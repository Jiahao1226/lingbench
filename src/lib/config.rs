// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Configuration loader for LingBench.
//!
//! Parses `lingbench.toml` configuration file for VMM benchmarking.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_workdir")]
    pub workdir: PathBuf,
    pub kernel: KernelConfig,
    pub rootfs: RootfsConfig,
    /// Root directory for VMM binaries. Relative to lingbench.toml location.
    #[serde(default = "default_vmm_dir")]
    pub vmm_dir: PathBuf,
    /// Per-VMM configuration.
    #[serde(rename = "vmm", default)]
    pub vmm_configs: Vec<VmmConfig>,
    #[serde(default)]
    pub logs: LogConfig,
    #[serde(default)]
    pub report: ReportConfig,
}

fn default_workdir() -> PathBuf {
    PathBuf::from("build")
}

fn default_vmm_dir() -> PathBuf {
    PathBuf::from("./vmm")
}

#[derive(Debug, Clone, Deserialize)]
pub struct KernelConfig {
    pub version: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub config_fragment: Option<PathBuf>,
    #[serde(default = "default_arch")]
    pub arch: String,
    #[serde(default)]
    pub extra_make_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RootfsConfig {
    #[serde(default = "default_containerfile")]
    pub containerfile: PathBuf,
    #[serde(default = "default_size_mib")]
    pub size_mib: u64,
    #[serde(default = "default_builder")]
    pub builder: String,
    #[serde(default)]
    pub formats: Vec<RootfsFormat>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RootfsFormat {
    Tar,
    Ext4,
    Cpio,
}

fn default_arch() -> String {
    "x86_64".into()
}

fn default_containerfile() -> PathBuf {
    PathBuf::from("guest/rootfs/Containerfile")
}

fn default_size_mib() -> u64 {
    512
}

fn default_builder() -> String {
    "podman".into()
}

/// VMM-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VmmConfig {
    pub name: String,
    pub binary: PathBuf,
    pub enabled: bool,
    #[serde(default)]
    pub runtime: Option<toml::Value>,
}

impl VmmConfig {
    /// Resolve binary path relative to vmm_dir.
    pub fn resolved_binary(&self, vmm_dir: &Path) -> PathBuf {
        if self.binary.is_absolute() {
            self.binary.clone()
        } else {
            vmm_dir.join(&self.binary)
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_log_dir")]
    pub dir: PathBuf,
}

fn default_log_level() -> String {
    "basic".to_string()
}

fn default_true() -> bool {
    true
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("./logs")
}

/// Report configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ReportConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_report_dir")]
    pub output_dir: PathBuf,
}

fn default_report_dir() -> PathBuf {
    PathBuf::from("./results")
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            enabled: default_true(),
            dir: default_log_dir(),
        }
    }
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            output_dir: default_report_dir(),
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let base = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let s = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let mut config: Config = toml::from_str(&s)
            .with_context(|| format!("parsing config {}", path.display()))?;

        // Resolve relative paths for logs and report output_dir
        config.logs.dir = base.join(&config.logs.dir);
        config.report.output_dir = base.join(&config.report.output_dir);

        Ok(config)
    }

    pub fn kernel_dir(&self) -> PathBuf {
        self.workdir.join("kernel")
    }

    pub fn rootfs_dir(&self) -> PathBuf {
        self.workdir.join("rootfs")
    }

    pub fn download_dir(&self) -> PathBuf {
        self.workdir.join("downloads")
    }

    /// Returns the list of VMM configs that are enabled.
    pub fn get_enabled_vmm(&self) -> Vec<&VmmConfig> {
        self.vmm_configs.iter().filter(|v| v.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    static CNT: AtomicU32 = AtomicU32::new(0);

    fn test_config(content: &str) -> Config {
        let n = CNT.fetch_add(1, Ordering::SeqCst);
        let path_str = format!("/tmp/lingbench_test_{n}.toml");
        let path = Path::new(&path_str);
        std::fs::write(path, content).unwrap();
        Config::load(path).unwrap()
    }

    #[test]
    fn test_load_two_vmm() {
        let content = "workdir = \"./build\"\n[kernel]\nversion = \"6.12.20\"\narch = \"x86_64\"\n[rootfs]\ncontainerfile = \"guest/rootfs/Containerfile\"\nsize_mib = 512\nbuilder = \"podman\"\nformats = [\"ext4\"]\nvmm_dir = \"./vmm\"\n[[vmm]]\nname = \"firecracker\"\nbinary = \"/usr/bin/firecracker\"\nenabled = true\n[[vmm]]\nname = \"cloud-hypervisor\"\nbinary = \"/usr/bin/cloud-hypervisor\"\nenabled = true\n[logs]\nlevel = \"basic\"\n[report]\nenabled = true";
        let config = test_config(content);
        let enabled = config.get_enabled_vmm();
        assert_eq!(enabled.len(), 2);
        assert_eq!(enabled[0].name, "firecracker");
        assert_eq!(enabled[1].name, "cloud-hypervisor");
    }

    #[test]
    fn test_get_enabled_vmm() {
        // Only firecracker is explicitly enabled; crosvm uses explicit false.
        let content = "workdir = \"./build\"\n[kernel]\nversion = \"6.12.20\"\narch = \"x86_64\"\n[rootfs]\ncontainerfile = \"guest/rootfs/Containerfile\"\nsize_mib = 512\nbuilder = \"podman\"\nformats = [\"ext4\"]\nvmm_dir = \"./vmm\"\n[[vmm]]\nname = \"firecracker\"\nbinary = \"/usr/bin/firecracker\"\nenabled = true\n[[vmm]]\nname = \"crosvm\"\nbinary = \"/usr/bin/crosvm\"\nenabled = false\n[logs]\nlevel = \"basic\"\n[report]\nenabled = true";
        let config = test_config(content);
        let enabled = config.get_enabled_vmm();
        eprintln!("vmm[0] {} enabled={}", config.vmm_configs[0].name, config.vmm_configs[0].enabled);
        eprintln!("vmm[1] {} enabled={}", config.vmm_configs[1].name, config.vmm_configs[1].enabled);
        eprintln!("enabled.len()={}", enabled.len());
        // Both are explicitly set; firecracker=true, crosvm=false
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "firecracker");
        assert_eq!(config.vmm_configs[0].enabled, true);
        assert_eq!(config.vmm_configs[1].enabled, false);
    }
}
