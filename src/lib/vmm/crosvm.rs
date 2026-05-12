// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use anyhow::{Result, bail};
use super::{VmConfig, VmmRunner, VmInstance};

pub struct CrosvmRunner {
    binary_path: PathBuf,
}

impl CrosvmRunner {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }
}

impl VmmRunner for CrosvmRunner {
    fn name(&self) -> &str {
        "crosvm"
    }

    fn detect(binary: &Path) -> Result<Self> {
        // Crosvm doesn't support --version, just check binary exists
        if !binary.exists() {
            bail!("Crosvm binary not found");
        }
        Ok(Self::new(binary.to_path_buf()))
    }

    fn spawn(&self, config: &VmConfig) -> Result<Box<dyn VmInstance>> {
        // Use unique socket path to avoid conflicts
        let id = format!("{}-{}", std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        let socket = config.socket_path.clone()
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/crosvm-{}.sock", id)));

        let serial_log = PathBuf::from(format!("/tmp/crosvm-{}-serial.log", id));
        let serial_file = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&serial_log)?;

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("run")
           .arg("--disable-sandbox")
           .arg(&config.kernel)
           .arg("--block").arg(format!("path={},root=true", config.rootfs.display()))
           .arg("-p").arg(format!(
               "console=ttyS0 reboot=k panic=1 root=/dev/vda rw quiet lingbench.scenario={}",
               config.scenario
           ))
           .arg("--cpus").arg("2")
           .arg("--mem").arg("512")
           .arg("--serial").arg(format!("type=file,path={}", serial_log.display()))
           .stdout(serial_file.try_clone()?)
           .stderr(serial_file);

        let child = cmd.spawn()?;

        Ok(Box::new(CrosvmInstance {
            child,
            _socket: socket,
            serial_log,
        }))
    }

    fn probe(binary: &Path) -> bool {
        binary.file_name()
            .map(|n| n.to_string_lossy().contains("crosvm"))
            .unwrap_or(false)
    }
}

pub struct CrosvmInstance {
    child: Child,
    _socket: PathBuf,
    serial_log: PathBuf,
}

impl VmInstance for CrosvmInstance {
    fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
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

    #[test]
    fn test_crosvm_probe() {
        assert!(CrosvmRunner::probe(Path::new("crosvm")));
        assert!(CrosvmRunner::probe(Path::new("crosvm_test")));
        assert!(!CrosvmRunner::probe(Path::new("firecracker")));
    }
}
