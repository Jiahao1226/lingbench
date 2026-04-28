// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::{Command, Child, Stdio};
use std::os::unix::process::CommandExt;
use anyhow::{Result, bail};
use super::{VmConfig, VmmRunner, VmInstance};

pub struct StratovirtRunner {
    binary_path: PathBuf,
}

impl StratovirtRunner {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }
}

impl VmmRunner for StratovirtRunner {
    fn name(&self) -> &str {
        "stratovirt"
    }

    fn detect(binary: &Path) -> Result<Self> {
        // Stratovirt doesn't support --version, just check binary exists
        if !binary.exists() {
            bail!("Stratovirt binary not found");
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
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/stratovirt-{}-qmp.sock", id)));

        let serial_log = PathBuf::from(format!("/tmp/stratovirt-{}-serial.log", id));

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("-kernel").arg(&config.kernel)
           .arg("-append").arg(format!(
               "console=ttyS0 reboot=k panic=1 root=/dev/vda rw quiet lingbench.scenario={}",
               config.scenario
           ))
           .arg("-cpu").arg("host")
           .arg("-m").arg("512")
           .arg("-smp").arg("2")
           .arg("-device").arg("virtio-blk-device,id=blk0,drive=hd0")
           .arg("-drive").arg(format!("file={},id=hd0,if=none", config.rootfs.display()))
           .arg("-serial").arg(format!("file,path={}", serial_log.display()))
           .arg("-qmp").arg(format!("unix:{},server,nowait", socket.display()))
           .stdout(Stdio::null())
           .stderr(Stdio::null())
           .process_group(0);

        let child = cmd.spawn()?;

        Ok(Box::new(StratovirtInstance {
            child,
            _socket: socket,
            serial_log,
        }))
    }

    fn probe(binary: &Path) -> bool {
        binary.file_name()
            .map(|n| n.to_string_lossy().contains("stratovirt"))
            .unwrap_or(false)
    }
}

pub struct StratovirtInstance {
    child: Child,
    _socket: PathBuf,
    serial_log: PathBuf,
}

impl VmInstance for StratovirtInstance {
    fn is_running(&mut self) -> bool {
        // First check if process has exited
        if self.child.try_wait().ok().flatten().is_some() {
            return false;
        }

        // Check if serial log contains completion marker
        // This handles the case where VM has completed but process hasn't exited yet
        if let Some(output) = self.get_serial_output() {
            if output.contains("LINGBENCH_RESULT_END") {
                return false;
            }
        }

        true
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
    fn test_stratovirt_probe() {
        assert!(StratovirtRunner::probe(Path::new("stratovirt")));
        assert!(StratovirtRunner::probe(Path::new("stratovirt_test")));
        assert!(!StratovirtRunner::probe(Path::new("firecracker")));
    }
}
