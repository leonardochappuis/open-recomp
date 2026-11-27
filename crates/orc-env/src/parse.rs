//! Pure parsers for the text that OS tools print. Kept separate from probing
//! so every format can be unit tested on any OS.

use crate::{Gpu, GpuKind, Vendor};

/// `/etc/os-release` -> (ID, VERSION_ID). Arch has no VERSION_ID.
pub fn os_release(text: &str) -> (Option<String>, Option<String>) {
    let mut id = None;
    let mut version = None;
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').to_string();
            match k.trim() {
                "ID" => id = Some(v),
                "VERSION_ID" => version = Some(v),
                _ => {}
            }
        }
    }
    (id, version)
}

/// `/proc/cpuinfo` -> (vendor, model name, logical cores).
/// x86 has `vendor_id` + `model name`; ARM boards often only `CPU implementer`.
pub fn cpuinfo(text: &str) -> (Option<Vendor>, Option<String>, Option<u32>) {
    let mut vendor = None;
    let mut model = None;
    let mut count = 0u32;
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "processor" => count += 1,
            "vendor_id" if vendor.is_none() => {
                vendor = Some(match v {
                    "AuthenticAMD" | "HygonGenuine" => Vendor::Amd,
                    "GenuineIntel" => Vendor::Intel,
                    _ => Vendor::Other,
                })
            }
            "CPU implementer" if vendor.is_none() => {
                vendor = Some(match v {
                    "0x41" => Vendor::Arm,
                    "0x51" => Vendor::Qualcomm,
                    "0x61" => Vendor::Apple,
                    _ => Vendor::Other,
                })
            }
            "model name" | "Model" if model.is_none() && !v.is_empty() => model = Some(clean(v)),
            _ => {}
        }
    }
    (vendor, model, (count > 0).then_some(count))
}

/// `/proc/meminfo` -> total RAM in bytes.
pub fn meminfo_total_bytes(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// Round bytes to whole GiB. Installed RAM reports slightly under the
/// marketed size (firmware reservations), so round to nearest.
pub fn bytes_to_gb(bytes: u64) -> u64 {
    const GIB: u64 = 1 << 30;
    (bytes + GIB / 2) / GIB
}

/// PCI / Vulkan vendor id -> vendor.
pub fn vendor_from_id(id: u32) -> Vendor {
    match id {
        0x10de => Vendor::Nvidia,
        0x1002 | 0x1022 => Vendor::Amd,
        0x8086 => Vendor::Intel,
        0x106b => Vendor::Apple,
        0x5143 => Vendor::Qualcomm,
        0x13b5 => Vendor::Arm,
        0x1414 => Vendor::Microsoft,
        0x10005 => Vendor::Mesa,
        _ => Vendor::Other,
    }
}

/// Best-effort vendor from a marketing name or vendor string.
pub fn vendor_from_name(name: &str) -> Vendor {
    let n = name.to_ascii_lowercase();
    if n.contains("nvidia") || n.contains("geforce") || n.contains("quadro") {
        Vendor::Nvidia
    } else if n.contains("amd") || n.contains("radeon") || n.contains("advanced micro devices") || n.contains("ati ") {
        Vendor::Amd
    } else if n.contains("intel") {
        Vendor::Intel
    } else if n.contains("apple") {
        Vendor::Apple
    } else if n.contains("qualcomm") || n.contains("adreno") {
        Vendor::Qualcomm
    } else if n.contains("microsoft") {
        Vendor::Microsoft
    } else {
        Vendor::Other
    }
}

/// `vulkaninfo --summary` -> GPUs. Ignores device/driver UUIDs on purpose.
pub fn vulkaninfo_summary(text: &str) -> Vec<Gpu> {
    let Some(start) = text.find("\nDevices:") else { return Vec::new() };
    let mut gpus = Vec::new();
    let mut cur: Option<VkDev> = None;
    for line in text[start..].lines() {
        let t = line.trim();
        if t.starts_with("GPU") && t.ends_with(':') {
            if let Some(d) = cur.take() {
                gpus.extend(d.finish());
            }
            cur = Some(VkDev::default());
            continue;
        }
        let (Some(d), Some((k, v))) = (cur.as_mut(), t.split_once('=')) else { continue };
        let v = v.trim().to_string();
        match k.trim() {
            "apiVersion" => d.api = Some(v),
            "driverVersion" => d.driver_version = Some(v),
            "driverInfo" => d.driver_info = Some(v),
            "vendorID" => d.vendor = u32::from_str_radix(v.trim_start_matches("0x"), 16).ok(),
            "deviceType" => d.kind = Some(v),
            "deviceName" => d.name = Some(v),
            _ => {}
        }
    }
    if let Some(d) = cur {
        gpus.extend(d.finish());
    }
    dedup(gpus)
}

#[derive(Default)]
struct VkDev {
    api: Option<String>,
    driver_version: Option<String>,
    driver_info: Option<String>,
    vendor: Option<u32>,
    kind: Option<String>,
    name: Option<String>,
}

impl VkDev {
    fn finish(self) -> Option<Gpu> {
        let model = clean(&self.name?);
        let kind = match self.kind.as_deref() {
            Some("PHYSICAL_DEVICE_TYPE_DISCRETE_GPU") => GpuKind::Discrete,
            Some("PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU") => GpuKind::Integrated,
            Some("PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU") => GpuKind::Virtual,
            Some("PHYSICAL_DEVICE_TYPE_CPU") => GpuKind::Software,
            _ => GpuKind::Unknown,
        };
        let vendor = match self.vendor.map(vendor_from_id) {
            Some(Vendor::Other) | None => vendor_from_name(&model),
            Some(v) => v,
        };
        let driver = self.driver_info.filter(|s| !s.is_empty()).or(self.driver_version);
        Some(Gpu { vendor, model, kind, driver, vulkan: self.api, vram_mb: None })
    }
}

/// Windows: output of
/// `Get-CimInstance Win32_VideoController | Select-Object Name,AdapterCompatibility,DriverVersion | ConvertTo-Json`.
/// PowerShell emits an object for one adapter and an array for several.
pub fn windows_video_controllers(json: &str) -> Vec<Gpu> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let items = match v {
        serde_json::Value::Array(a) => a,
        o @ serde_json::Value::Object(_) => vec![o],
        _ => return Vec::new(),
    };
    let gpus = items
        .iter()
        .filter_map(|it| {
            let model = clean(it.get("Name")?.as_str()?);
            let compat = it.get("AdapterCompatibility").and_then(|s| s.as_str()).unwrap_or("");
            let vendor = match vendor_from_name(compat) {
                Vendor::Other => vendor_from_name(&model),
                v => v,
            };
            let raw = it.get("DriverVersion").and_then(|s| s.as_str());
            let kind = if model.contains("Basic Render") || model.contains("Basic Display") {
                GpuKind::Software
            } else {
                GpuKind::Unknown
            };
            let driver = raw.map(|r| match vendor {
                Vendor::Nvidia => nvidia_windows_driver(r).unwrap_or_else(|| r.to_string()),
                _ => r.to_string(),
            });
            Some(Gpu { vendor, model, kind, driver, vulkan: None, vram_mb: None })
        })
        .collect();
    dedup(gpus)
}

/// NVIDIA's Windows driver "32.0.15.6094" is marketed as "560.94":
/// the last five digits of the last two groups.
pub fn nvidia_windows_driver(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let digits = format!("{}{:0>4}", parts[2], parts[3]);
    if digits.len() < 5 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let last5 = &digits[digits.len() - 5..];
    Some(format!("{}.{}", &last5[..3], &last5[3..]))
}

/// macOS: `system_profiler SPDisplaysDataType -json`.
pub fn macos_displays(json: &str) -> Vec<Gpu> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let Some(items) = v.get("SPDisplaysDataType").and_then(|a| a.as_array()) else { return Vec::new() };
    let gpus = items
        .iter()
        .filter_map(|it| {
            let model = clean(it.get("sppci_model")?.as_str()?);
            let vendor_raw = it.get("spdisplays_vendor").and_then(|s| s.as_str()).unwrap_or("");
            let vendor = match vendor_from_name(vendor_raw.trim_start_matches("sppci_vendor_")) {
                Vendor::Other => vendor_from_name(&model),
                v => v,
            };
            let bus = it.get("sppci_bus").and_then(|s| s.as_str()).unwrap_or("");
            let kind = if vendor == Vendor::Apple || bus.contains("builtin") {
                GpuKind::Integrated
            } else if bus.contains("pcie") {
                GpuKind::Discrete
            } else {
                GpuKind::Unknown
            };
            let vram_mb = it
                .get("spdisplays_vram")
                .or_else(|| it.get("spdisplays_vram_shared"))
                .and_then(|s| s.as_str())
                .and_then(parse_size_mb);
            Some(Gpu { vendor, model, kind, driver: None, vulkan: None, vram_mb })
        })
        .collect();
    dedup(gpus)
}

/// "8 GB" / "1536 MB" -> MB.
pub fn parse_size_mb(s: &str) -> Option<u64> {
    let mut it = s.split_whitespace();
    let n: u64 = it.next()?.parse().ok()?;
    match it.next()?.to_ascii_uppercase().as_str() {
        "GB" => Some(n * 1024),
        "MB" => Some(n),
        _ => None,
    }
}

/// Same GPU can show up once per driver (e.g. RADV and AMDVLK). Keep the first.
fn dedup(gpus: Vec<Gpu>) -> Vec<Gpu> {
    let mut out: Vec<Gpu> = Vec::new();
    for g in gpus {
        if !out.iter().any(|o| o.vendor == g.vendor && o.model == g.model) {
            out.push(g);
        }
    }
    out
}

fn clean(s: &str) -> String {
    s.replace("(R)", "").replace("(TM)", "").split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const VK: &str = "==========\nVULKANINFO\n==========\n\nDevices:\n========\nGPU0:\n\tapiVersion         = 1.4.357\n\tdriverVersion      = 620.73.4.0\n\tvendorID           = 0x10de\n\tdeviceID           = 0x220a\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_DISCRETE_GPU\n\tdeviceName         = NVIDIA GeForce RTX 3080\n\tdriverID           = DRIVER_ID_NVIDIA_PROPRIETARY\n\tdriverName         = NVIDIA\n\tdriverInfo         = 620.73.04\n\tdeviceUUID         = b3790ed3-954d-b5da-d9b9-1d5cf9c30f9e\nGPU1:\n\tapiVersion         = 1.4.363\n\tdriverVersion      = 26.3.1\n\tvendorID           = 0x10005\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_CPU\n\tdeviceName         = llvmpipe (LLVM 23.1.5, 256 bits)\n\tdriverInfo         = Mesa 26.3.1-arch1.2 (LLVM 23.1.5)\nGPU2:\n\tapiVersion         = 1.4.300\n\tvendorID           = 0x1002\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU\n\tdeviceName         = AMD Radeon Graphics (RADV RAPHAEL_MENDOCINO)\n\tdriverInfo         = Mesa 26.3.1\n";

    #[test]
    fn vulkaninfo_devices() {
        let g = vulkaninfo_summary(VK);
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].vendor, Vendor::Nvidia);
        assert_eq!(g[0].kind, GpuKind::Discrete);
        assert_eq!(g[0].model, "NVIDIA GeForce RTX 3080");
        assert_eq!(g[0].driver.as_deref(), Some("620.73.04"));
        assert_eq!(g[0].vulkan.as_deref(), Some("1.4.357"));
        assert_eq!(g[1].kind, GpuKind::Software);
        assert_eq!(g[2].vendor, Vendor::Amd);
        assert_eq!(g[2].kind, GpuKind::Integrated);
        // No UUIDs leak into the output.
        assert!(!serde_json::to_string(&g).unwrap().contains("b3790ed3"));
    }

    #[test]
    fn vulkaninfo_without_devices_section() {
        assert!(vulkaninfo_summary("ERROR: no ICD found").is_empty());
    }

    #[test]
    fn os_release_quotes_and_missing_version() {
        assert_eq!(os_release("NAME=\"Arch Linux\"\nID=arch\n"), (Some("arch".into()), None));
        assert_eq!(os_release("ID=ubuntu\nVERSION_ID=\"24.04\"\n"), (Some("ubuntu".into()), Some("24.04".into())));
    }

    #[test]
    fn cpuinfo_x86_and_arm() {
        let x86 = "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 9 9950X3D 16-Core Processor\nprocessor\t: 1\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 9 9950X3D 16-Core Processor\n";
        assert_eq!(cpuinfo(x86), (Some(Vendor::Amd), Some("AMD Ryzen 9 9950X3D 16-Core Processor".into()), Some(2)));
        let arm = "processor\t: 0\nCPU implementer\t: 0x41\nprocessor\t: 1\nCPU implementer\t: 0x41\nModel\t: Raspberry Pi 5 Model B Rev 1.0\n";
        assert_eq!(cpuinfo(arm), (Some(Vendor::Arm), Some("Raspberry Pi 5 Model B Rev 1.0".into()), Some(2)));
    }

    #[test]
    fn meminfo_and_rounding() {
        let b = meminfo_total_bytes("MemTotal:       63153752 kB\nMemFree: 1 kB\n").unwrap();
        assert_eq!(bytes_to_gb(b), 60);
        assert_eq!(bytes_to_gb(16 * (1 << 30) - 300 * (1 << 20)), 16);
    }

    #[test]
    fn windows_single_object_and_nvidia_driver() {
        let one =
            r#"{"Name":"NVIDIA GeForce RTX 4070","AdapterCompatibility":"NVIDIA","DriverVersion":"32.0.15.6094"}"#;
        let g = windows_video_controllers(one);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].vendor, Vendor::Nvidia);
        assert_eq!(g[0].driver.as_deref(), Some("560.94"));

        let many = r#"[{"Name":"AMD Radeon(TM) Graphics","AdapterCompatibility":"Advanced Micro Devices, Inc.","DriverVersion":"31.0.21912.14"},
                       {"Name":"Microsoft Basic Render Driver","AdapterCompatibility":"Microsoft","DriverVersion":"10.0.26100.1"}]"#;
        let g = windows_video_controllers(many);
        assert_eq!(g[0].vendor, Vendor::Amd);
        assert_eq!(g[0].model, "AMD Radeon Graphics");
        assert_eq!(g[1].kind, GpuKind::Software);
    }

    #[test]
    fn nvidia_driver_edge_cases() {
        assert_eq!(nvidia_windows_driver("31.0.15.3623").as_deref(), Some("536.23"));
        assert_eq!(nvidia_windows_driver("garbage"), None);
    }

    #[test]
    fn macos_apple_silicon() {
        let j = r#"{"SPDisplaysDataType":[{"sppci_model":"Apple M3 Pro","spdisplays_vendor":"sppci_vendor_Apple","sppci_bus":"spdisplays_builtin","sppci_cores":"18"}]}"#;
        let g = macos_displays(j);
        assert_eq!(g[0].vendor, Vendor::Apple);
        assert_eq!(g[0].kind, GpuKind::Integrated);

        let intel_mac = r#"{"SPDisplaysDataType":[{"sppci_model":"AMD Radeon Pro 5500M","spdisplays_vendor":"sppci_vendor_amd","sppci_bus":"spdisplays_pcie_device","spdisplays_vram":"8 GB"}]}"#;
        let g = macos_displays(intel_mac);
        assert_eq!(g[0].vendor, Vendor::Amd);
        assert_eq!(g[0].kind, GpuKind::Discrete);
        assert_eq!(g[0].vram_mb, Some(8192));
    }
}
