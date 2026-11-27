//! OS-specific probing. Every external tool is optional: if it is missing or
//! fails, the value stays unknown instead of erroring.

use crate::{parse, Cpu, Environment, Memory, Os, SCHEMA_VERSION};
use std::process::Command;

pub fn collect() -> Environment {
    let mut env = Environment {
        schema: SCHEMA_VERSION,
        os: Os { family: std::env::consts::OS.to_string(), ..Default::default() },
        cpu: Cpu { arch: std::env::consts::ARCH.to_string(), ..Default::default() },
        gpus: Vec::new(),
        memory: Memory::default(),
    };
    platform(&mut env);
    if env.cpu.logical_cores.is_none() {
        env.cpu.logical_cores = std::thread::available_parallelism().ok().and_then(|n| u32::try_from(n.get()).ok());
    }
    // Vulkan gives the best GPU + driver info on Linux and Windows (and on
    // macOS through MoltenVK). Fill in or enrich what the OS reported.
    if let Some(out) = run("vulkaninfo", &["--summary"]) {
        merge_vulkan(&mut env, parse::vulkaninfo_summary(&out));
    }
    if env.gpus.is_empty() {
        env.gpus = fallback_gpus();
    }
    env
}

fn merge_vulkan(env: &mut Environment, vk: Vec<crate::Gpu>) {
    for v in vk {
        match env.gpus.iter_mut().find(|g| g.vendor == v.vendor) {
            Some(g) => {
                g.vulkan = v.vulkan.clone();
                if g.kind == crate::GpuKind::Unknown {
                    g.kind = v.kind;
                }
                if g.driver.is_none() {
                    g.driver = v.driver.clone();
                }
            }
            None => env.gpus.push(v),
        }
    }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(target_os = "linux")]
fn platform(env: &mut Environment) {
    use std::fs::read_to_string;
    if let Ok(t) = read_to_string("/etc/os-release") {
        let (id, ver) = parse::os_release(&t);
        env.os.name = id;
        env.os.version = ver;
    }
    env.os.kernel = read_to_string("/proc/sys/kernel/osrelease").ok().map(|s| s.trim().to_string());
    if let Ok(t) = read_to_string("/proc/cpuinfo") {
        let (vendor, model, cores) = parse::cpuinfo(&t);
        env.cpu.vendor = vendor;
        env.cpu.model = model;
        env.cpu.logical_cores = cores;
    }
    env.memory.ram_gb =
        read_to_string("/proc/meminfo").ok().and_then(|t| parse::meminfo_total_bytes(&t)).map(parse::bytes_to_gb);
    // GPUs come from vulkaninfo in collect(), with sysfs as the fallback.
}

/// Used when neither the OS probe nor vulkaninfo found any GPU.
#[cfg(not(target_os = "linux"))]
fn fallback_gpus() -> Vec<crate::Gpu> {
    Vec::new()
}

/// Linux without vulkan-tools: vendor ids from sysfs, model unknown.
#[cfg(target_os = "linux")]
fn fallback_gpus() -> Vec<crate::Gpu> {
    let mut gpus: Vec<crate::Gpu> = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/drm") else { return gpus };
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let vendor_hex = std::fs::read_to_string(e.path().join("device/vendor")).unwrap_or_default();
        let Ok(id) = u32::from_str_radix(vendor_hex.trim().trim_start_matches("0x"), 16) else { continue };
        let vendor = parse::vendor_from_id(id);
        let driver = (vendor == crate::Vendor::Nvidia)
            .then(|| std::fs::read_to_string("/proc/driver/nvidia/version").ok())
            .flatten()
            .and_then(|t| t.split_whitespace().find(|w| w.matches('.').count() == 2).map(String::from));
        if !gpus.iter().any(|g| g.vendor == vendor) {
            gpus.push(crate::Gpu {
                vendor,
                model: format!("{vendor:?} GPU (install vulkan-tools for the model name)"),
                kind: crate::GpuKind::Unknown,
                driver,
                vulkan: None,
                vram_mb: None,
            });
        }
    }
    gpus
}

#[cfg(target_os = "windows")]
fn platform(env: &mut Environment) {
    let ps = |q: &str| run("powershell", &["-NoProfile", "-NonInteractive", "-Command", q]);
    if let Some(j) = ps("Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version | ConvertTo-Json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&j) {
            env.os.name = v["Caption"].as_str().map(|s| s.trim_start_matches("Microsoft ").to_string());
            env.os.version = v["Version"].as_str().map(String::from);
        }
    }
    if let Some(j) =
        ps("Get-CimInstance Win32_Processor | Select-Object -First 1 Name,NumberOfLogicalProcessors | ConvertTo-Json")
    {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&j) {
            env.cpu.model = v["Name"].as_str().map(|s| s.trim().to_string());
            env.cpu.vendor = env.cpu.model.as_deref().map(parse::vendor_from_name);
            env.cpu.logical_cores = v["NumberOfLogicalProcessors"].as_u64().and_then(|n| u32::try_from(n).ok());
        }
    }
    if let Some(j) = ps("(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory") {
        env.memory.ram_gb = j.trim().parse::<u64>().ok().map(parse::bytes_to_gb);
    }
    if let Some(j) = ps("Get-CimInstance Win32_VideoController | Select-Object Name,AdapterCompatibility,DriverVersion | ConvertTo-Json") {
        env.gpus = parse::windows_video_controllers(&j);
    }
}

#[cfg(target_os = "macos")]
fn platform(env: &mut Environment) {
    env.os.name = Some("macOS".into());
    env.os.version = run("sw_vers", &["-productVersion"]).map(|s| s.trim().to_string());
    env.cpu.model = run("sysctl", &["-n", "machdep.cpu.brand_string"]).map(|s| s.trim().to_string());
    env.cpu.vendor = env.cpu.model.as_deref().map(parse::vendor_from_name);
    env.memory.ram_gb =
        run("sysctl", &["-n", "hw.memsize"]).and_then(|s| s.trim().parse::<u64>().ok()).map(parse::bytes_to_gb);
    if let Some(j) = run("system_profiler", &["SPDisplaysDataType", "-json"]) {
        env.gpus = parse::macos_displays(&j);
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn platform(_env: &mut Environment) {}
