// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

pub mod scenario;

pub use scenario::{ScenarioRunner, RunResult};

/// Shared interrupt flag type for Ctrl+C graceful shutdown
pub type InterruptFlag = std::sync::Arc<std::sync::atomic::AtomicBool>;
