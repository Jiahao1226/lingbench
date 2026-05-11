// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use super::{MetricBatch, HostMessage};

pub struct MetricsCollector {
    data: Arc<Mutex<HashMap<String, Vec<MetricBatch>>>>,
    logs_enabled: bool,
    logs_dir: PathBuf,
}

impl MetricsCollector {
    pub fn new(logs_enabled: bool, logs_dir: PathBuf) -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            logs_enabled,
            logs_dir,
        }
    }

    /// Receive and store metrics (sync version)
    pub fn ingest(&self, batch: MetricBatch) -> Result<(), String> {
        let key = format!("{}_{}", batch.vmm, batch.scenario);

        // Write to log file (optional)
        if self.logs_enabled {
            let filename = format!("metrics_{}.jsonl", key);
            let path = self.logs_dir.join(&filename);
            if let Ok(json) = serde_json::to_string(&batch) {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", json)
                    });
            }
        }

        let mut data = self.data.lock().unwrap();
        data.entry(key.clone())
            .or_insert_with(Vec::new)
            .push(batch);

        Ok(())
    }

    /// Parse JSON message and store (sync version)
    pub fn ingest_json(&self, json: &str) -> Result<(), String> {
        match serde_json::from_str::<HostMessage>(json) {
            Ok(msg) => {
                if let Some(batch) = msg.into_metric_batch() {
                    self.ingest(batch)?;
                }
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get all collected metrics (sync version)
    pub fn get_all(&self) -> HashMap<String, Vec<MetricBatch>> {
        self.data.lock().unwrap().clone()
    }

    /// Query by VMM and Scenario (sync version)
    pub fn query(&self, vmm: &str, scenario: &str) -> Option<Vec<MetricBatch>> {
        let data = self.data.lock().unwrap();
        data.get(&format!("{}_{}", vmm, scenario)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{Metric, RunStatus};

    #[test]
    fn test_collector_store() {
        let collector = MetricsCollector::new(false, PathBuf::from("/tmp"));

        let batch = MetricBatch {
            batch_id: 1,
            scenario: "cpu-sysbench".into(),
            vmm: "firecracker".into(),
            metrics: vec![
                Metric { name: "events_per_second".into(), value: 5599.21, ts: 0 }
            ],
            status: RunStatus::Running,
        };

        collector.ingest(batch).unwrap();

        let result = collector.query("firecracker", "cpu-sysbench");
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_collector_query_not_found() {
        let collector = MetricsCollector::new(false, PathBuf::from("/tmp"));
        let result = collector.query("unknown", "unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_ingest_json() {
        let collector = MetricsCollector::new(false, PathBuf::from("/tmp"));

        let json = r#"{"type":"metric","batch":1,"scenario":"cpu-sysbench","vmm":"firecracker","metrics":[{"name":"events_per_second","value":5599.21,"ts":1713844800000}]}"#;

        collector.ingest_json(json).unwrap();

        let result = collector.query("firecracker", "cpu-sysbench");
        assert!(result.is_some());
        assert_eq!(result.unwrap()[0].metrics[0].value, 5599.21);
    }

    #[test]
    fn test_message_serialization() {
        let json = r#"{"type":"metric","batch":1,"scenario":"cpu-sysbench","vmm":"firecracker","metrics":[{"name":"events_per_second","value":5599.21,"ts":1713844800000}]}"#;
        let msg: HostMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, HostMessage::Metric { .. }));
    }
}
