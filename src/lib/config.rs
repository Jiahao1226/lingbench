// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Configuration loader for LingBench V2.
//!
//! Parses `lingbench.toml` configuration file for VMM benchmarking.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

/// Main configuration structure for LingBench V2.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub global: GlobalConfig,
    #[serde(rename = "vmm")]
    pub vmm_configs: Vec<VmmConfig>,
    pub logs: LogConfig,
    pub report: ReportConfig,
}

/// Global configuration settings.
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalConfig {
    pub workdir: PathBuf,
    pub kernel: PathBuf,
    pub rootfs: PathBuf,
}

/// VMM-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VmmConfig {
    pub name: String,
    pub binary: PathBuf,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub runtime: Option<toml::Value>,
}

fn default_enabled() -> bool {
    true
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

/// Report generation configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ReportConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_report_dir")]
    pub output_dir: PathBuf,
}

fn default_report_dir() -> PathBuf {
    PathBuf::from("./reports")
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get all enabled VMM configurations.
    pub fn get_enabled_vmm(&self) -> Vec<&VmmConfig> {
        self.vmm_configs.iter().filter(|v| v.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(content: &str) -> Config {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("lingbench_test_{}.toml", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::write(&path, content).unwrap();
        let config = Config::load(&path).unwrap();
        std::fs::remove_file(&path).ok();
        config
    }

    #[test]
    fn test_load_two_vmm() {
        let config = test_config(
            r#"
[global]
workdir = "./build"
kernel = "./build/kernel/bzImage"
rootfs = "./build/rootfs.ext4"

[[vmm]]
name = "firecracker"
binary = "/usr/bin/firecracker"
enabled = true

[[vmm]]
name = "cloud-hypervisor"
binary = "/usr/bin/cloud-hypervisor"
enabled = true

[logs]
level = "basic"
enabled = true

[report]
enabled = true
"#,
        );
        assert_eq!(config.vmm_configs.len(), 2);
        assert_eq!(config.vmm_configs[0].name, "firecracker");
        assert_eq!(config.vmm_configs[1].name, "cloud-hypervisor");
    }

    #[test]
    fn test_get_enabled_vmm() {
        let config = test_config(
            r#"
[global]
workdir = "./build"
kernel = "./build/kernel/bzImage"
rootfs = "./build/rootfs.ext4"

[[vmm]]
name = "firecracker"
binary = "/usr/bin/firecracker"
enabled = true

[[vmm]]
name = "crosvm"
binary = "/usr/bin/crosvm"
enabled = false

[logs]
[report]
"#,
        );
        let enabled = config.get_enabled_vmm();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "firecracker");
    }
}
