//! Coarse, privacy-safe hardware/software profile for open-recomp.
//!
//! Recomp bugs are often hardware specific (NVIDIA vs AMD vs Intel vs Apple
//! unified memory, Windows vs Linux vs macOS, Vulkan vs D3D12 vs Metal,
//! driver versions). Every finding, attempt and test result references one of
//! these profiles. The profile is collected by this crate, never typed by an
//! agent, so it can't be hallucinated.
//!
//! Privacy: only coarse values. No hostnames, usernames, serial numbers,
//! MAC addresses or device UUIDs.

pub mod parse;
mod probe;

use serde::{Deserialize, Serialize};

/// Bumped when fields change meaning.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub schema: u32,
    pub os: Os,
    pub cpu: Cpu,
    pub gpus: Vec<Gpu>,
    pub memory: Memory,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Os {
    /// linux, windows, macos, ...
    pub family: String,
    /// Distro or edition, e.g. "arch", "Windows 11 Pro", "macOS".
    pub name: Option<String>,
    /// Distro/OS version, e.g. "24.04", "10.0.26100", "15.2".
    pub version: Option<String>,
    /// Kernel release on Linux.
    pub kernel: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cpu {
    /// x86_64, aarch64, ...
    pub arch: String,
    pub vendor: Option<Vendor>,
    pub model: Option<String>,
    pub logical_cores: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gpu {
    pub vendor: Vendor,
    pub model: String,
    pub kind: GpuKind,
    /// Human-readable driver version, e.g. "620.73.04" or "Mesa 26.3.1".
    pub driver: Option<String>,
    /// Highest Vulkan version the driver reports, if Vulkan is available.
    pub vulkan: Option<String>,
    pub vram_mb: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Qualcomm,
    Arm,
    Microsoft,
    Mesa,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuKind {
    Discrete,
    Integrated,
    Virtual,
    /// Software rasterizer (llvmpipe, WARP, ...).
    Software,
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    /// Rounded to the nearest GiB to stay coarse.
    pub ram_gb: Option<u64>,
    /// CPU and GPU share one memory pool (Apple Silicon, APU-only systems).
    pub unified: bool,
}

impl Environment {
    /// Probe the current machine. Never fails: unknown values stay `None`.
    pub fn collect() -> Self {
        let mut env = probe::collect();
        // Drop software rasterizers unless they are all we have.
        if !env_has_no_hw_gpu(&env.gpus) {
            env.gpus.retain(|g| g.kind != GpuKind::Software);
        }
        env.memory.unified = is_unified(&env);
        env
    }

    /// The GPUs a game would realistically run on (no software rasterizers).
    pub fn hardware_gpus(&self) -> impl Iterator<Item = &Gpu> {
        self.gpus.iter().filter(|g| g.kind != GpuKind::Software)
    }

    /// Stable short id for deduplicating identical profiles on the server.
    /// FNV-1a over the canonical JSON, so it doesn't change between Rust
    /// versions the way `DefaultHasher` may.
    pub fn fingerprint(&self) -> String {
        let json = serde_json::to_vec(self).expect("Environment always serializes");
        let mut h: u64 = 0xcbf29ce484222325;
        for b in json {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100000001b3);
        }
        format!("{h:016x}")
    }

    /// One-line summary, e.g. "linux/arch, AMD Ryzen 9 9950X3D, nvidia RTX 3080 (620.73.04)".
    pub fn summary(&self) -> String {
        let os = match &self.os.name {
            Some(n) => format!("{}/{}", self.os.family, n),
            None => self.os.family.clone(),
        };
        let cpu = self.cpu.model.clone().unwrap_or_else(|| self.cpu.arch.clone());
        let gpus: Vec<String> = self
            .hardware_gpus()
            .map(|g| match &g.driver {
                Some(d) => format!("{} ({d})", g.model),
                None => g.model.clone(),
            })
            .collect();
        let gpus = if gpus.is_empty() { "no GPU detected".into() } else { gpus.join(" + ") };
        let unified = if self.memory.unified { ", unified memory" } else { "" };
        format!("{os}, {cpu}, {gpus}{unified}")
    }
}

fn env_has_no_hw_gpu(gpus: &[Gpu]) -> bool {
    gpus.iter().all(|g| g.kind == GpuKind::Software)
}

fn is_unified(env: &Environment) -> bool {
    if env.os.family == "macos" && env.cpu.arch == "aarch64" {
        return true;
    }
    let mut hw = env.hardware_gpus().peekable();
    hw.peek().is_some() && hw.all(|g| g.kind == GpuKind::Integrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu(kind: GpuKind) -> Gpu {
        Gpu { vendor: Vendor::Amd, model: "x".into(), kind, driver: None, vulkan: None, vram_mb: None }
    }

    fn env(family: &str, arch: &str, gpus: Vec<Gpu>) -> Environment {
        Environment {
            schema: SCHEMA_VERSION,
            os: Os { family: family.into(), ..Default::default() },
            cpu: Cpu { arch: arch.into(), ..Default::default() },
            gpus,
            memory: Memory::default(),
        }
    }

    #[test]
    fn apple_silicon_is_unified() {
        assert!(is_unified(&env("macos", "aarch64", vec![])));
        assert!(!is_unified(&env("macos", "x86_64", vec![gpu(GpuKind::Discrete)])));
    }

    #[test]
    fn apu_only_is_unified_but_dgpu_is_not() {
        assert!(is_unified(&env("linux", "x86_64", vec![gpu(GpuKind::Integrated)])));
        assert!(!is_unified(&env("linux", "x86_64", vec![gpu(GpuKind::Integrated), gpu(GpuKind::Discrete)])));
        assert!(!is_unified(&env("linux", "x86_64", vec![gpu(GpuKind::Software)])));
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive() {
        let a = env("linux", "x86_64", vec![gpu(GpuKind::Discrete)]);
        let mut b = a.clone();
        assert_eq!(a.fingerprint(), b.fingerprint());
        b.gpus[0].driver = Some("1.0".into());
        assert_ne!(a.fingerprint(), b.fingerprint());
    }
}
