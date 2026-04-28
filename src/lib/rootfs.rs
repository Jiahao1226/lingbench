// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Rootfs building functionality.
//!
//! NOTE: This module is pending update for V2 config structure.

use anyhow::Result;

/// Rootfs image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootfsFormat {
    Tar,
    Ext4,
    Cpio,
}

/// Build the guest rootfs.
///
/// NOTE: This is a stub pending V2 config integration.
/// The V2 Config uses simple PathBuf for rootfs path instead of
/// the V1 RootfsConfig structure with containerfile/builder/fields.
pub fn build(_cfg: &crate::Config) -> Result<()> {
    unimplemented!("rootfs build requires V2 config update")
}
