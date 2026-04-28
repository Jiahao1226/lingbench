// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

pub fn download(url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    tracing::info!("downloading {}", url);
    let mut resp = reqwest::blocking::get(url)
        .with_context(|| format!("GET {url}"))?
        .error_for_status()?;
    let tmp = dest.with_extension("part");
    let mut file = std::fs::File::create(&tmp)?;
    resp.copy_to(&mut file)?;
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    let digest = hasher.finalize();
    let got: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    if got != expected.to_lowercase() {
        bail!(
            "sha256 mismatch for {}: expected {}, got {}",
            path.display(),
            expected,
            got
        );
    }
    Ok(())
}

pub fn run(cmd: &mut Command) -> Result<()> {
    tracing::info!("$ {:?}", cmd);
    let status = cmd
        .status()
        .with_context(|| format!("spawning {:?}", cmd))?;
    if !status.success() {
        bail!("command failed: {:?} ({})", cmd, status);
    }
    Ok(())
}

pub fn run_capture(cmd: &mut Command) -> Result<String> {
    tracing::info!("$ {:?}", cmd);
    let out = cmd
        .output()
        .with_context(|| format!("spawning {:?}", cmd))?;
    if !out.status.success() {
        bail!(
            "command failed: {:?} ({})\nstderr: {}",
            cmd,
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}
