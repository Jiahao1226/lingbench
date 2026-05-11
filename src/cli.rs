// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lingbench")]
pub enum Command {
    /// Run VMM benchmark
    Run {
        /// VMM names to run (all enabled if not specified, comma-separated)
        #[arg(long, value_delimiter = ',')]
        vmm: Option<Vec<String>>,

        /// Scenarios to run (all if not specified, "all" or comma-separated)
        #[arg(long, value_delimiter = ',')]
        scenario: Option<Vec<String>>,

        /// Output directory
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Generate report only (using existing data)
    Report {
        /// Input results.json path (from --save-results)
        #[arg(long)]
        input: Option<PathBuf>,

        /// Report output path
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// List available configurations
    List {
        #[arg(long)]
        vmm: bool,

        #[arg(long)]
        scenario: bool,
    },

    /// Build guest image
    Build {
        #[command(subcommand)]
        target: BuildTarget,
    },
}

#[derive(Subcommand)]
pub enum BuildTarget {
    /// Build kernel
    Kernel,
    /// Build rootfs
    Rootfs,
    /// Build all
    All,
}
