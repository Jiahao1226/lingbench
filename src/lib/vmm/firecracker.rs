// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use anyhow::{Result, bail};

use super::{VmConfig, VmmRunner, VmInstance};

pub struct FirecrackerRunner {
    binary_path: PathBuf,
}

impl FirecrackerRunner {
    pub fn new(binary_path: PathBuf) -> Self {
        Self { binary_path }
    }
}

impl VmmRunner for FirecrackerRunner {
    fn name(&self) -> &str {
        "firecracker"
    }

    fn detect(binary: &Path) -> Result<Self> {
        let output = Command::new(binary)
            .arg("--version")
            .output()?;
        if !output.status.success() {
            bail!("Firecracker binary not valid");
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
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/fc-{}.sock", id)));
        let log_path = PathBuf::from(format!("/tmp/fc-{}-serial.log", id));

        // Clean up old socket and log
        let _ = fs::remove_file(&socket);
        let _ = fs::remove_file(&log_path);

        // 1. Start Firecracker daemon
        // Note: In Firecracker API mode, guest serial outputs to stdout/stderr
        // We redirect it to log file to capture
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;

        Command::new(&self.binary_path)
            .arg("--api-sock").arg(&socket)
            .arg("--no-seccomp")
            .stdout(log_file.try_clone()?)
            .stderr(log_file)
            .spawn()?;

        // 2. Wait for socket ready
        let max_wait = 50;
        let mut waited = 0;
        while waited < max_wait {
            if socket.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            waited += 1;
        }

        if !socket.exists() {
            bail!("Firecracker socket not created");
        }

        // Firecracker requires vmlinux format kernel, not bzImage
        // Convert /path/to/kernel/bzImage -> /path/to/kernel/linux-6.12.20/vmlinux
        let kernel_path = if config.kernel.to_string_lossy().ends_with("bzImage") {
            if let Some(parent) = config.kernel.parent() {
                parent.join("linux-6.12.20/vmlinux")
            } else {
                config.kernel.clone()
            }
        } else {
            config.kernel.clone()
        };

        // 3. Configure boot source
        let boot_result = Command::new("curl")
            .args(&[
                "-N", "--unix-socket", socket.to_str().unwrap(),
                "-X", "PUT",
                "http://localhost/boot-source",
                "-H", "Content-Type: application/json",
                "-d", &format!(
                    "{{\"kernel_image_path\": \"{}\", \"boot_args\": \"console=ttyS0 root=/dev/vda rw lingbench.scenario={}\"}}",
                    kernel_path.display(),
                    config.scenario
                ),
            ])
            .output()?;

        if !boot_result.status.success() {
            bail!("Failed to set boot source: {}", String::from_utf8_lossy(&boot_result.stderr));
        }

        // 4. Configure drive (root device)
        let drive_result = Command::new("curl")
            .args(&[
                "-N", "--unix-socket", socket.to_str().unwrap(),
                "-X", "PUT",
                "http://localhost/drives/root",
                "-H", "Content-Type: application/json",
                "-d", &format!(
                    "{{\"drive_id\": \"root\", \"path_on_host\": \"{}\", \"is_root_device\": true, \"is_read_only\": false}}",
                    config.rootfs.display()
                ),
            ])
            .output()?;

        if !drive_result.status.success() {
            bail!("Failed to configure drive: {}", String::from_utf8_lossy(&drive_result.stderr));
        }

        // 5. Configure machine (vCPU, memory)
        let machine_result = Command::new("curl")
            .args(&[
                "-N", "--unix-socket", socket.to_str().unwrap(),
                "-X", "PUT",
                "http://localhost/machine-config",
                "-H", "Content-Type: application/json",
                "-d", "{\"vcpu_count\": 1, \"mem_size_mib\": 512}",
            ])
            .output()?;

        if !machine_result.status.success() {
            bail!("Failed to configure machine: {}", String::from_utf8_lossy(&machine_result.stderr));
        }

        // 6. Start instance
        let start_result = Command::new("curl")
            .args(&[
                "-N", "--unix-socket", socket.to_str().unwrap(),
                "-X", "PUT",
                "http://localhost/actions",
                "-H", "Content-Type: application/json",
                "-d", "{\"action_type\": \"InstanceStart\"}",
            ])
            .output()?;

        if !start_result.status.success() {
            bail!("Failed to start instance: {}", String::from_utf8_lossy(&start_result.stderr));
        }

        Ok(Box::new(FirecrackerInstance {
            socket,
            log_path,
        }))
    }

    fn probe(binary: &Path) -> bool {
        binary.file_name()
            .map(|n| n.to_string_lossy().contains("firecracker"))
            .unwrap_or(false)
    }
}

pub struct FirecrackerInstance {
    socket: PathBuf,
    log_path: PathBuf,
}

impl VmInstance for FirecrackerInstance {
    fn is_running(&mut self) -> bool {
        self.socket.exists()
    }

    fn wait(&mut self) -> Result<i32> {
        // Wait for VM to run
        std::thread::sleep(std::time::Duration::from_secs(1));
        Ok(0)
    }

    fn kill(&mut self) -> Result<()> {
        // Shutdown via API
        let _ = Command::new("curl")
            .args(&[
                "--unix-socket", self.socket.to_str().unwrap(),
                "-X", "PUT",
                "http://localhost/actions",
                "-H", "Content-Type: application/json",
                "-d", "{\"action_type\": \"InstanceHalt\"}",
            ])
            .output();

        // Kill firecracker process
        let _ = Command::new("pkill")
            .arg("firecracker")
            .output();

        // Clean up socket
        let _ = fs::remove_file(&self.socket);

        Ok(())
    }

    fn get_serial_output(&self) -> Option<String> {
        // Use from_utf8_lossy to handle non-UTF-8 characters in serial log
        std::fs::read(&self.log_path)
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firecracker_probe() {
        assert!(FirecrackerRunner::probe(Path::new("firecracker")));
        assert!(FirecrackerRunner::probe(Path::new("firecracker_test")));
        assert!(!FirecrackerRunner::probe(Path::new("crosvm")));
    }
}
