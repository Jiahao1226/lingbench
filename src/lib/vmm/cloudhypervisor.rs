// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Cloud-Hypervisor VMM runner implementation.

use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use anyhow::{Result, bail};

use super::traits::{VmConfig, VmInstance, VmmRunner};

pub struct CloudHypervisorRunner {
    binary_path: PathBuf,
}

impl CloudHypervisorRunner {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }
    
    /// Static name accessor for registry
    pub fn name_static() -> &'static str {
        "cloud-hypervisor"
    }
}

impl VmmRunner for CloudHypervisorRunner {
    fn name(&self) -> &str { "cloud-hypervisor" }
    
    fn detect(binary: &Path) -> Result<Self> {
        let output = Command::new(binary)
            .arg("--version")
            .output()?;
        if !output.status.success() {
            bail!("Cloud-Hypervisor binary not valid");
        }
        Ok(Self::new(binary.to_path_buf()))
    }
    
    fn spawn(&self, config: &VmConfig) -> Result<Box<dyn VmInstance>> {
        // Use unique socket path to avoid conflicts (based on PID + timestamp)
        let id = format!("{}-{}", std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        let socket = config.socket_path.clone()
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/ch-{}.sock", id)));
        
        let serial_log = PathBuf::from(format!("/tmp/ch-{}-serial.log", id));
        let serial_file = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&serial_log)?;

        // Build command
        // Cloud-Hypervisor uses --api-socket instead of --api-sock
        // Disk param format: path={},readonly=off,image_type=raw
        // Memory format: size=2147483648 (bytes)
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--api-socket").arg(&socket)
           .arg("--kernel").arg(&config.kernel)
           .arg("--disk").arg(format!("path={},readonly=off,image_type=raw", config.rootfs.display()))
           .arg("--cmdline").arg(format!("root=/dev/vda rw lingbench.scenario={}", config.scenario))
           .arg("--cpus").arg("boot=1")
           .arg("--memory").arg("size=2147483648")
           .arg("--serial").arg(format!("file={}", serial_log.display()))
           .stdout(serial_file.try_clone()?)
           .stderr(serial_file);
        
        let child = cmd.spawn()?;
        
        Ok(Box::new(CloudHypervisorInstance {
            child,
            _socket: socket,
            serial_log,
        }))
    }
    
    fn probe(binary: &Path) -> bool {
        binary.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.contains("cloud-hypervisor"))
            .unwrap_or(false)
    }
}

pub struct CloudHypervisorInstance {
    child: Child,
    _socket: PathBuf,
    serial_log: PathBuf,
}

impl VmInstance for CloudHypervisorInstance {
    fn is_running(&mut self) -> bool {
        let status = self.child.try_wait();
        status.ok().flatten().is_none()
    }
    
    fn wait(&mut self) -> Result<i32> {
        let status = self.child.wait()?;
        Ok(status.code().unwrap_or(-1))
    }
    
    fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }
    
    fn get_serial_output(&self) -> Option<String> {
        // Use from_utf8_lossy to handle non-UTF-8 characters in serial log
        std::fs::read(&self.serial_log)
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_cloudhypervisor_probe() {
        let result = CloudHypervisorRunner::probe(Path::new("cloud-hypervisor"));
        assert!(result);
        
        let result = CloudHypervisorRunner::probe(Path::new("firecracker"));
        assert!(!result);
    }
}
