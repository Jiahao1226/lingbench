// SPDX-FileCopyrightText: 2026 LingCage
//
// SPDX-License-Identifier: Apache-2.0

//! Core library for the lingbench VMM test framework.
//!
//! Builds guest artifacts (kernel + rootfs) today, and is the home for
//! the host-side VMM runner and result parser as they land. Kept as a
//! library so non-CLI consumers — tests, benches, future TUIs — don't
//! pull in clap or tracing-subscriber.

pub mod config;
pub mod kernel;
pub mod rootfs;
pub mod util;

pub use config::{Config, KernelConfig, RootfsConfig, RootfsFormat};
