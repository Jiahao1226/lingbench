// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use super::types::{Metric, MetricBatch, RunStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HostMessage {
    #[serde(rename = "metric")]
    Metric {
        batch: u64,
        scenario: String,
        vmm: String,
        metrics: Vec<Metric>,
    },
    #[serde(rename = "status")]
    Status {
        scenario: String,
        vmm: String,
        status: String,
        progress: u8,
    },
    #[serde(rename = "complete")]
    Complete {
        scenario: String,
        vmm: String,
        status: String,
        duration_ms: u64,
    },
}

impl HostMessage {
    pub fn into_metric_batch(self) -> Option<MetricBatch> {
        match self {
            HostMessage::Metric { batch, scenario, vmm, metrics } => Some(MetricBatch {
                batch_id: batch,
                scenario,
                vmm,
                metrics,
                status: RunStatus::Running,
            }),
            _ => None,
        }
    }
}
