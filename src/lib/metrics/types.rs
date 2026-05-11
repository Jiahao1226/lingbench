// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub ts: u64,
}

impl Metric {
    /// Format value for display: use compact notation for large numbers
    pub fn formatted_value(&self) -> String {
        if self.value >= 1_000_000.0 {
            format!("{:.1}M", self.value / 1_000_000.0)
        } else if self.value >= 1_000.0 {
            format!("{:.1}k", self.value / 1_000.0)
        } else {
            format!("{:.1}", self.value)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Completed { duration_ms: u64 },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBatch {
    pub batch_id: u64,
    pub scenario: String,
    pub vmm: String,
    pub metrics: Vec<Metric>,
    pub status: RunStatus,
}
