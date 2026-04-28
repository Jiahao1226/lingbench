// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Scenario Runner - Run VMM + scenario combination tests

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use anyhow::Result;
use regex::Regex;
use crate::config::Config;
use crate::vmm::{VmmRegistry, VmConfig};
use crate::metrics::{MetricsCollector, Metric, MetricBatch, RunStatus};
use crate::runner::InterruptFlag;

const SEPARATOR_EQ: &str = "========================================";
const SEPARATOR_DASH: &str = "----------------------------------------";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunResult {
    pub vmm: String,
    pub scenario: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub metrics: Vec<Metric>,  // Actual benchmark metrics
}

pub struct ScenarioRunner {
    config: Config,
    vmm_registry: VmmRegistry,
    collector: Option<Arc<MetricsCollector>>,
    config_dir: PathBuf,
    interrupt_flag: Option<InterruptFlag>,
}

impl ScenarioRunner {
    pub fn new(config: Config, vmm_registry: VmmRegistry, collector: Option<Arc<MetricsCollector>>, config_dir: PathBuf) -> Self {
        Self { config, vmm_registry, collector, config_dir, interrupt_flag: None }
    }

    /// Set the interrupt flag for graceful shutdown
    pub fn set_interrupt_flag(&mut self, flag: InterruptFlag) {
        self.interrupt_flag = Some(flag);
    }

    /// Check if interrupt flag is set
    fn is_interrupted(&self) -> bool {
        self.interrupt_flag.as_ref()
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// Parse metrics from serial output
    fn parse_metrics(&self, output: &str, scenario: &str) -> Vec<Metric> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        match scenario {
            "cpu-sysbench" => {
                if let Ok(re) = Regex::new(r"events per second:\s*([0-9.]+)") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            return vec![Metric {
                                name: "events_per_second".to_string(),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            "cpu-coremark" => {
                if let Ok(re) = Regex::new(r"(?:CoreMark 1.0:|Iterations/Sec\s*:)\s*([0-9.]+)") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            return vec![Metric {
                                name: "score".to_string(),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            "cpu-stress" => {
                if let Ok(re) = Regex::new(r"^.*cpu\s+\d+\s+[\d.]+\s+[\d.]+\s+[\d.]+\s+(\d+\.\d+)\s+(\d+\.\d+)\s*$") {
                    for line in output.lines() {
                        if line.contains("stress-ng: metrc") && line.contains("cpu") {
                            if let Some(caps) = re.captures(line) {
                                if let Some(val_str) = caps.get(1) {
                                    if let Ok(val) = val_str.as_str().parse::<f64>() {
                                        return vec![Metric {
                                            name: "bogo_ops".to_string(),
                                            value: val,
                                            ts: timestamp,
                                        }];
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "mem-sysbench" => {
                if let Ok(re) = Regex::new(r"Total operations:.*?\(([0-9.]+)\s*per second\)") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            return vec![Metric {
                                name: "ops_per_second".to_string(),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
                if let Ok(re) = Regex::new(r"ops/s:\s*([0-9.]+)") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            return vec![Metric {
                                name: "ops_per_second".to_string(),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            "io-randread" | "io-randwrite" | "io-seqread" => {
                if let Ok(re) = Regex::new(r"IOPS=\s*([0-9k.]+)") {
                    if let Some(caps) = re.captures(output) {
                        let val_str = &caps[1];
                        let val_str = val_str.trim_end_matches('k');
                        if let Ok(val) = val_str.parse::<f64>() {
                            let final_val = if caps[1].ends_with('k') {
                                val * 1000.0
                            } else {
                                val
                            };
                            return vec![Metric {
                                name: "iops".to_string(),
                                value: final_val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            "meminfo" => {
                let mut metrics = Vec::new();
                if let Ok(re) = Regex::new(r"MemTotal:\s*([0-9]+)\s*kB") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            metrics.push(Metric {
                                name: "MemTotal_KB".to_string(),
                                value: val,
                                ts: timestamp,
                            });
                        }
                    }
                }
                if let Ok(re) = Regex::new(r"MemFree:\s*([0-9]+)\s*kB") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            metrics.push(Metric {
                                name: "MemFree_KB".to_string(),
                                value: val,
                                ts: timestamp,
                            });
                        }
                    }
                }
                if !metrics.is_empty() {
                    return metrics;
                }
            }
            "app-redis" => {
                if let Ok(re) = Regex::new(r"(GET|SET|HSET):\s*([0-9.]+)\s*requests per second") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[2].parse::<f64>() {
                            return vec![Metric {
                                name: format!("{}_req_per_sec", &caps[1]),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            "app-nginx" => {
                if let Ok(re) = Regex::new(r"Requests/sec:\s*([0-9.]+)") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            return vec![Metric {
                                name: "requests_per_sec".to_string(),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            "app-memcached" => {
                if let Ok(re) = Regex::new(r"STAT curr_connections\s+([0-9]+)") {
                    if let Some(caps) = re.captures(output) {
                        if let Ok(val) = caps[1].parse::<f64>() {
                            return vec![Metric {
                                name: "curr_connections".to_string(),
                                value: val,
                                ts: timestamp,
                            }];
                        }
                    }
                }
            }
            _ => {}
        }
        vec![]
    }

    /// Run a single VMM + scenario
    pub fn run_one(&self, vmm_name: &str, scenario: &str) -> Result<RunResult> {
        let start = Instant::now();

        // 0. Check interrupt flag before doing anything
        if self.is_interrupted() {
            return Ok(RunResult {
                vmm: vmm_name.to_string(),
                scenario: scenario.to_string(),
                success: false,
                error: Some("Interrupted".to_string()),
                duration_ms: Some(start.elapsed().as_millis() as u64),
                metrics: vec![],
            });
        }

        // 1. Get VMM config
        let _vmm_config = self.config.vmm_configs.iter()
            .find(|v| v.name == vmm_name)
            .ok_or_else(|| anyhow::anyhow!("VMM {} not found in config", vmm_name))?;

        // 2. Get VMM Runner
        let runner = self.vmm_registry.get(vmm_name)
            .ok_or_else(|| anyhow::anyhow!("VMM runner {} not registered", vmm_name))?;

        // 3. Build VM config (resolve relative paths to absolute paths)
        let workdir = self.config_dir
            .join(&self.config.global.workdir)
            .canonicalize()
            .unwrap_or_else(|_| self.config_dir.join(&self.config.global.workdir));
        
        fn resolve_path(workdir: &Path, p: &Path) -> PathBuf {
            if p.is_absolute() {
                return p.to_path_buf();
            }
            workdir.join(p).canonicalize().unwrap_or_else(|_| workdir.join(p))
        }
        
        let kernel = resolve_path(&workdir, &self.config.global.kernel);
        let rootfs = resolve_path(&workdir, &self.config.global.rootfs);
        let vm_config = VmConfig {
            kernel,
            rootfs,
            scenario: scenario.to_string(),
            socket_path: None,
        };

        // 4. Check interrupt flag before spawning
        if self.is_interrupted() {
            return Ok(RunResult {
                vmm: vmm_name.to_string(),
                scenario: scenario.to_string(),
                success: false,
                error: Some("Interrupted".to_string()),
                duration_ms: Some(start.elapsed().as_millis() as u64),
                metrics: vec![],
            });
        }

        // 5. Spawn VM
        let mut instance = match runner.spawn(&vm_config) {
            Ok(inst) => inst,
            Err(e) => {
                println!("{}", SEPARATOR_EQ);
                println!("[VMM Failed] {} | Error: {}", vmm_name, e);
                println!("{}", SEPARATOR_EQ);
                return Ok(RunResult {
                    vmm: vmm_name.to_string(),
                    scenario: scenario.to_string(),
                    success: false,
                    error: Some(format!("VMM spawn failed: {}", e)),
                    duration_ms: Some(start.elapsed().as_millis() as u64),
                    metrics: vec![],
                });
            }
        };

        // 6. Wait for test completion - detect via Serial Marker
        let is_app_scenario = scenario.starts_with("app-");
        let timeout_secs = if is_app_scenario { 300 } else { 120 };
        let loop_start = Instant::now();
        let mut did_complete = false;
        let mut did_timeout = false;
        let mut last_progress_print = Instant::now();

        println!("{} | {} | running", vmm_name, scenario);

        loop {
            // Check interrupt flag
            if self.is_interrupted() {
                println!("\nReceived Ctrl+C, cleaning up...");
                let _ = instance.kill();
                return Ok(RunResult {
                    vmm: vmm_name.to_string(),
                    scenario: scenario.to_string(),
                    success: false,
                    error: Some("Interrupted".to_string()),
                    duration_ms: Some(start.elapsed().as_millis() as u64),
                    metrics: vec![],
                });
            }

            // Check timeout
            if loop_start.elapsed().as_secs() >= timeout_secs {
                did_timeout = true;
                break;
            }

            // Check serial output for completion
            if let Some(output) = instance.get_serial_output() {
                if output.contains("LINGBENCH_RESULT_END") {
                    did_complete = true;
                    break;
                }
            }

            // Check if process exited
            if !instance.is_running() {
                if let Some(output) = instance.get_serial_output() {
                    if output.contains("LINGBENCH_RESULT_END") {
                        did_complete = true;
                        break;
                    }
                }
                // Process exited but marker not found - wait for serial buffer to flush
                // Guest often reboots/powerdowns right after test completes, need longer wait
                std::thread::sleep(Duration::from_millis(5000));
                if let Some(output) = instance.get_serial_output() {
                    if output.contains("LINGBENCH_RESULT_END") {
                        did_complete = true;
                        break;
                    }
                }
                break;
            }

            // Print progress every 5 seconds so user knows we're still alive
            if last_progress_print.elapsed().as_secs() >= 5 {
                let elapsed = loop_start.elapsed().as_secs();
                println!("{} | {} | running ({}s)", vmm_name, scenario, elapsed);
                last_progress_print = Instant::now();
            }

            std::thread::sleep(Duration::from_millis(1));
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // 8. Read serial output and parse metrics
        let serial_output = instance.get_serial_output().unwrap_or_default();
        let metrics = self.parse_metrics(&serial_output, scenario);

        // 7. Print result with metrics
        if did_complete {
            if metrics.is_empty() {
                println!("{} | {} | ✓ | {}ms", vmm_name, scenario, duration_ms);
            } else {
                let metric_strs: Vec<String> = metrics.iter()
                    .map(|m| format!("{}={}", m.name, m.formatted_value()))
                    .collect();
                println!("{} | {} | ✓ | {}ms | {}", vmm_name, scenario, duration_ms, metric_strs.join(", "));
            }
        } else if did_timeout {
            println!("{} | {} | ✗ | Timeout", vmm_name, scenario);
        } else {
            println!("{} | {} | ✗ | Process exited", vmm_name, scenario);
        }
        println!("{}", SEPARATOR_DASH);

        // 9. Store metrics to collector
        if let Some(ref collector) = self.collector {
            let batch = MetricBatch {
                batch_id: 1,
                scenario: scenario.to_string(),
                vmm: vmm_name.to_string(),
                metrics: metrics.clone(),
                status: if did_complete {
                    RunStatus::Completed { duration_ms }
                } else {
                    RunStatus::Error { message: if did_timeout { "Timeout".to_string() } else { "Process exited".to_string() } }
                },
            };
            let _ = collector.ingest(batch);
        }

        // 10. Cleanup - kill and wait to ensure process fully exits and releases locks
        let _ = instance.kill();
        let _ = instance.wait();

        Ok(RunResult {
            vmm: vmm_name.to_string(),
            scenario: scenario.to_string(),
            success: did_complete,
            error: if did_timeout { Some("Timeout".to_string()) } else if !did_complete { Some("Process exited".to_string()) } else { None },
            duration_ms: Some(duration_ms),
            metrics,
        })
    }

    /// Get list of all available scenarios
    pub fn get_all_scenarios(&self) -> Vec<String> {
        vec![
            "cpu-sysbench".to_string(),
            "cpu-coremark".to_string(),
            "cpu-stress".to_string(),
            "mem-sysbench".to_string(),
            "meminfo".to_string(),
            "io-randread".to_string(),
            "io-randwrite".to_string(),
            "io-seqread".to_string(),
            "app-redis".to_string(),
            "app-nginx".to_string(),
            "app-memcached".to_string(),
        ]
    }

    /// Run batch tests
    pub fn run_batch(
        &self,
        vmm_names: Option<Vec<String>>,
        scenarios: Option<Vec<String>>,
    ) -> Result<Vec<RunResult>> {
        let vmm_list = vmm_names.unwrap_or_else(|| {
            let list = self.config.get_enabled_vmm()
                .iter()
                .map(|v| v.name.clone())
                .collect();
            eprintln!("DEBUG: No VMM specified, using enabled VMMs: {:?}", list);
            list
        });

        let scenario_list = scenarios.unwrap_or_else(|| vec![
            "cpu-sysbench".to_string(),
            "cpu-coremark".to_string(),
            "cpu-stress".to_string(),
            "mem-sysbench".to_string(),
            "meminfo".to_string(),
            "io-randread".to_string(),
            "io-randwrite".to_string(),
            "io-seqread".to_string(),
            "app-redis".to_string(),
            "app-nginx".to_string(),
            "app-memcached".to_string(),
        ]);

        let mut results = Vec::new();

        for vmm_name in &vmm_list {
            let mut vmm_failed = false;

            // Check interrupt before starting this VMM
            if self.is_interrupted() {
                break;
            }

            for scenario in &scenario_list {
                let result = self.run_one(vmm_name, scenario)?;
                
                if result.error.as_ref().map(|e| e.contains("VMM spawn failed")).unwrap_or(false) {
                    vmm_failed = true;
                }
                if result.error.as_ref().map(|e| e == "Interrupted").unwrap_or(false) {
                    // Was interrupted mid-run
                    results.push(result);
                    break;
                }
                
                results.push(result);

                if vmm_failed {
                    break;
                }

                // Wait between scenarios
                std::thread::sleep(Duration::from_millis(1000));
            }

            // Print VMM finished
            if !vmm_failed && !self.is_interrupted() {
                println!("{}", SEPARATOR_EQ);
                println!("[VMM Finished] {}", vmm_name);
                println!("{}", SEPARATOR_EQ);
            }

            // Wait between VMMs
            std::thread::sleep(Duration::from_secs(2));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_result_structure() {
        let result = RunResult {
            vmm: "firecracker".to_string(),
            scenario: "cpu-sysbench".to_string(),
            success: true,
            error: None,
            duration_ms: Some(1000),
        };

        assert_eq!(result.vmm, "firecracker");
        assert!(result.success);
    }
}
