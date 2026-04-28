// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! VMM Runner trait definitions.

use std::path::{Path, PathBuf};
use anyhow::Result;

/// VMM startup configuration
#[derive(Debug, Clone)]
pub struct VmConfig {
    pub kernel: PathBuf,
    pub rootfs: PathBuf,
    pub scenario: String,
    pub socket_path: Option<PathBuf>,
}

/// VMM Runner Trait - each VMM implements its own startup logic
pub trait VmmRunner: Send + Sync {
    /// VMM name
    fn name(&self) -> &str;
    
    /// Detect and create Runner from binary path
    fn detect(binary: &Path) -> Result<Self>
    where
        Self: Sized;
    
    /// Start VM, return instance
    fn spawn(&self, config: &VmConfig) -> Result<Box<dyn VmInstance>>;
    
    /// Check if binary matches this VMM type
    fn probe(binary: &Path) -> bool
    where
        Self: Sized;
}

/// Running VM instance
pub trait VmInstance: Send {
    /// Check if VM is still running
    fn is_running(&mut self) -> bool;
    
    /// Wait for VM to exit
    fn wait(&mut self) -> Result<i32>;
    
    /// Kill VM
    fn kill(&mut self) -> Result<()>;
    
    /// Get serial output content
    fn get_serial_output(&self) -> Option<String>;
}
