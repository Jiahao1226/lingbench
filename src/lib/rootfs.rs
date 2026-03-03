// SPDX-FileCopyrightText: 2026 LingCage
//
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};

use crate::config::{Config, RootfsFormat};
use crate::util;

const TAG: &str = "lingbench-rootfs:latest";

pub fn build(cfg: &Config) -> Result<()> {
    let rcfg = &cfg.rootfs;
    let rootfs_dir = cfg.rootfs_dir();
    std::fs::create_dir_all(&rootfs_dir)?;

    if !rcfg.containerfile.exists() {
        bail!("containerfile not found: {}", rcfg.containerfile.display());
    }

    let builder = &rcfg.builder;
    let context_dir = rcfg
        .containerfile
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    util::run(
        Command::new(builder)
            .arg("build")
            .arg("-f")
            .arg(&rcfg.containerfile)
            .arg("-t")
            .arg(TAG)
            .arg(&context_dir),
    )?;

    let cid = util::run_capture(
        Command::new(builder)
            .arg("create")
            .arg(TAG)
            .arg("/bin/true"),
    )?;
    let tar_path = rootfs_dir.join("rootfs.tar");
    tracing::info!("exporting {} -> {}", cid, tar_path.display());
    let export_status = Command::new(builder)
        .arg("export")
        .arg(&cid)
        .arg("-o")
        .arg(&tar_path)
        .status();
    let _ = Command::new(builder).arg("rm").arg(&cid).status();
    let export_status = export_status?;
    if !export_status.success() {
        bail!("{} export failed ({})", builder, export_status);
    }

    let formats = if rcfg.formats.is_empty() {
        vec![RootfsFormat::Ext4]
    } else {
        rcfg.formats.clone()
    };

    for fmt in formats {
        match fmt {
            RootfsFormat::Tar => {
                tracing::info!("rootfs (tar): {}", tar_path.display());
            }
            RootfsFormat::Ext4 => {
                let img = rootfs_dir.join("rootfs.ext4");
                make_ext4(&tar_path, &img, &rootfs_dir, rcfg.size_mib)?;
                tracing::info!("rootfs (ext4): {}", img.display());
            }
            RootfsFormat::Cpio => {
                let cpio = rootfs_dir.join("rootfs.cpio");
                make_cpio(&tar_path, &cpio, &rootfs_dir)?;
                tracing::info!("rootfs (cpio): {}", cpio.display());
            }
        }
    }
    Ok(())
}

fn stage_tar(tar: &Path, workdir: &Path, name: &str) -> Result<PathBuf> {
    let staging = workdir.join(name);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    util::run(
        Command::new("tar")
            .arg("-xf")
            .arg(tar)
            .arg("-C")
            .arg(&staging)
            .arg("--numeric-owner"),
    )?;
    Ok(staging)
}

fn make_ext4(tar: &Path, img: &Path, workdir: &Path, size_mib: u64) -> Result<()> {
    let staging = stage_tar(tar, workdir, "ext4-staging")?;
    if img.exists() {
        std::fs::remove_file(img)?;
    }
    util::run(
        Command::new("truncate")
            .arg("-s")
            .arg(format!("{size_mib}M"))
            .arg(img),
    )?;
    util::run(
        Command::new("mkfs.ext4")
            .arg("-F")
            .arg("-L")
            .arg("rootfs")
            .arg("-d")
            .arg(&staging)
            .arg(img),
    )?;
    std::fs::remove_dir_all(&staging)?;
    Ok(())
}

fn make_cpio(tar: &Path, cpio: &Path, workdir: &Path) -> Result<()> {
    let staging = stage_tar(tar, workdir, "cpio-staging")?;
    // make output absolute since we cd into staging
    if let Some(parent) = cpio.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if cpio.exists() {
        std::fs::remove_file(cpio)?;
    }
    std::fs::File::create(cpio)?;
    let cpio_abs = std::fs::canonicalize(cpio)?;
    let staging_abs = std::fs::canonicalize(&staging)?;
    let sh = format!(
        "cd '{}' && find . -print0 | cpio --null --create --format=newc > '{}'",
        staging_abs.display(),
        cpio_abs.display(),
    );
    util::run(Command::new("sh").arg("-c").arg(&sh))?;
    std::fs::remove_dir_all(&staging)?;
    Ok(())
}
