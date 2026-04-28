// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Kernel building functionality.
//!
//! NOTE: This module is pending update for V2 config structure.

use std::path::PathBuf;

use anyhow::Result;

/// Build the guest kernel.
///
/// NOTE: This is a stub pending V2 config integration.
/// The V2 Config uses simple PathBuf for kernel path instead of
/// the V1 KernelConfig structure with version/source_url fields.
pub fn build(_cfg: &crate::Config) -> Result<()> {
    unimplemented!("kernel build requires V2 config update")
}

/// Get the kernel artifact path for the given architecture.
pub fn artifact_path(arch: &str) -> PathBuf {
    match arch {
        "x86_64" | "i386" => PathBuf::from("arch/x86/boot/bzImage"),
        "arm64" | "aarch64" => PathBuf::from("arch/arm64/boot/Image"),
        _ => PathBuf::from("vmlinux"),
    }
}
