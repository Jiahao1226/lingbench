// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;

mod cli;

use cli::Command;
use lingbench::config::Config;
use lingbench::report::{generate_html_report, ChartData, ReportData, ScenarioRow, VmmResult, ScenarioResult, NamedMetric};
use lingbench::runner::{ScenarioRunner, RunResult};
use lingbench::vmm::VmmRegistry;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cmd = cli::Command::parse();
    match cmd {
        Command::Run {
            vmm,
            scenario,
            output,
        } => run_cmd(vmm, scenario, output),
        Command::Report { input, output } => report_cmd(input, output),
        Command::List { vmm, scenario } => list_cmd(vmm, scenario),
        Command::Build { target } => build_cmd(target),
    }
}

// ============ RUN COMMAND ============

fn run_cmd(
    vmm_names: Option<Vec<String>>,
    scenario_names: Option<Vec<String>>,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    let config = Config::load("lingbench.toml")?;

    // Filter VMMs
    let vmm_configs: Vec<_> = if let Some(ref names) = vmm_names {
        config.vmm_configs.iter().filter(|c| names.contains(&c.name)).collect()
    } else {
        config.vmm_configs.iter().collect()
    };

    if vmm_configs.is_empty() {
        anyhow::bail!("No VMMs selected");
    }

    // Filter scenarios
    let scenarios: Vec<String> = if let Some(ref names) = scenario_names {
        if names.iter().any(|n| n == "all") {
            get_all_scenarios()
        } else {
            names.clone()
        }
    } else {
        get_all_scenarios()
    };

    // Setup interrupt handler
    let interrupt_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&interrupt_flag);
    let flag_for_handler = Arc::clone(&interrupt_flag);

    ctrlc::set_handler(move || {
        flag_for_handler.store(true, Ordering::SeqCst);
    }).ok();

    // Create VMM registry
    let mut registry = VmmRegistry::new();
    for vmm_cfg in &vmm_configs {
        if let Err(e) = registry.detect_and_register(&vmm_cfg.binary) {
            eprintln!("Warning: Failed to detect VMM {} at {}: {}", vmm_cfg.name, vmm_cfg.binary.display(), e);
        }
    }

    // Create output directory
    let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("lingbench_results"));
    std::fs::create_dir_all(&output_dir)?;

    // Create scenario runner
    let mut runner = ScenarioRunner::new(
        config.clone(),
        registry,
        None,
        PathBuf::from("."),
    );
    runner.set_interrupt_flag(flag_clone);

    println!("Running benchmarks...");
    println!("VMMs: {:?}", vmm_configs.iter().map(|c| &c.name).collect::<Vec<_>>());
    println!("Scenarios: {:?}", scenarios);

    let results = runner.run_batch(Some(vmm_configs.into_iter().map(|c| c.name.clone()).collect()), Some(scenarios))?;

    // Save results
    let results_path = output_dir.join("results.json");
    let results_json = serde_json::to_string_pretty(&results)?;
    std::fs::write(&results_path, &results_json)?;
    println!("Results saved to {}", results_path.display());

    // Generate report
    let timestamp_str = format_timestamp(0);
    let report_path = output_dir.join(format!("lingbench_Results_{}.html", timestamp_str));

    let report_data = build_report_data(&results);
    generate_html_report(&report_data, &report_path, None)?;
    println!("Report generated: {}", report_path.display());

    Ok(())
}

// ============ REPORT COMMAND ============

fn report_cmd(input_path: Option<PathBuf>, output_path: Option<PathBuf>) -> Result<()> {
    let input_dir = PathBuf::from("lingbench_results");
    let results_path = input_path.unwrap_or_else(|| input_dir.join("results.json"));

    let content = std::fs::read_to_string(&results_path)
        .with_context(|| format!("Failed to read results from {}", results_path.display()))?;
    let results: Vec<RunResult> = serde_json::from_str(&content)?;

    let report_data = build_report_data(&results);

    let output_dir = PathBuf::from("lingbench_results");
    let timestamp_str = format_timestamp(0);
    let report_path = output_path.unwrap_or_else(|| output_dir.join(format!("lingbench_Results_{}.html", timestamp_str)));

    generate_html_report(&report_data, &report_path, None)?;
    println!("Report generated: {}", report_path.display());

    Ok(())
}

// ============ LIST COMMAND ============

fn list_cmd(list_vmm: bool, list_scenario: bool) -> Result<()> {
    let config = Config::load("lingbench.toml")?;

    if list_vmm {
        println!("Available VMMs:");
        for v in &config.vmm_configs {
            println!("  - {}", v.name);
        }
    }

    if list_scenario {
        println!("Available Scenarios:");
        for (name, meta) in SCENARIO_META {
            println!("  - {} ({})", name, meta.display_name);
        }
    }

    if !list_vmm && !list_scenario {
        println!("Available VMMs:");
        for v in &config.vmm_configs {
            println!("  - {}", v.name);
        }
        println!("\nAvailable Scenarios:");
        for (name, meta) in SCENARIO_META {
            println!("  - {} ({})", name, meta.display_name);
        }
    }

    Ok(())
}

// ============ BUILD COMMAND ============

fn build_cmd(target: cli::BuildTarget) -> Result<()> {
    let config = Config::load("lingbench.toml")?;
    
    match target {
        cli::BuildTarget::Kernel => {
            println!("Building kernel...");
            lingbench::kernel::build(&config)?;
            println!("Kernel build complete.");
        }
        cli::BuildTarget::Rootfs => {
            println!("Building rootfs...");
            lingbench::rootfs::build(&config)?;
            println!("Rootfs build complete.");
        }
        cli::BuildTarget::All => {
            println!("Building all...");
            lingbench::kernel::build(&config)?;
            lingbench::rootfs::build(&config)?;
            println!("All builds complete.");
        }
    }
    Ok(())
}

// ============ SCENARIO METADATA ============

#[allow(dead_code)]
struct ScenarioMeta {
    display_name: &'static str,
    unit: &'static str,
    subtitle: &'static str,
    tooltip: &'static str,
    run_cmd: &'static str,
}

const SCENARIO_META: &[(&str, ScenarioMeta)] = &[
    ("cpu-sysbench", ScenarioMeta {
        display_name: "CPU Sysbench",
        unit: "events/s",
        subtitle: "Tool: sysbench 1.0.20 | Test: 10s, 1 thread, primes ≤10000",
        tooltip: "CPU Sysbench: Measures CPU performance by calculating prime numbers.\nTool: sysbench 1.0.20\nTest: 10 seconds, 1 thread, prime numbers up to 10000\nCmd: sysbench cpu --threads=1 --time=10 run",
        run_cmd: "lingbench run --scenario cpu-sysbench --vmm <vmm>",
    }),
    ("cpu-coremark", ScenarioMeta {
        display_name: "CPU Coremark Score",
        unit: "CoreMark",
        subtitle: "Tool: CoreMark v1.0 | Test: Single-threaded",
        tooltip: "CPU Coremark: Standard CPU benchmark measuring integer and floating-point performance.\nTool: CoreMark v1.0\nTest: Single-threaded performance\nCmd: coremark",
        run_cmd: "lingbench run --scenario cpu-coremark --vmm <vmm>",
    }),
    ("cpu-stress", ScenarioMeta {
        display_name: "CPU Stress",
        unit: "bogo ops/s",
        subtitle: "Tool: stress 1.4.12 | Test: 10s, 1 worker",
        tooltip: "CPU Stress: Measures CPU stress using bogo operations.\nTool: stress 1.4.12\nTest: 10 seconds, 1 worker\nCmd: stress-ng --cpu 1 --timeout 10s --metrics-brief",
        run_cmd: "lingbench run --scenario cpu-stress --vmm <vmm>",
    }),
    ("mem-sysbench", ScenarioMeta {
        display_name: "Memory Sysbench",
        unit: "ops/s",
        subtitle: "Tool: sysbench 1.0.20 | Test: 4GB memory, sequential access",
        tooltip: "Memory Sysbench: Measures memory bandwidth and latency.\nTool: sysbench 1.0.20\nTest: 4GB memory, sequential access\nCmd: sysbench memory --memory-total-size=4G run",
        run_cmd: "lingbench run --scenario mem-sysbench --vmm <vmm>",
    }),
    ("meminfo", ScenarioMeta {
        display_name: "Memory Info",
        unit: "KB",
        subtitle: "Tool: /proc/meminfo | Test: Read memory info",
        tooltip: "Memory Info: Displays system memory information.\nTool: /proc/meminfo\nCmd: cat /proc/meminfo",
        run_cmd: "lingbench run --scenario meminfo --vmm <vmm>",
    }),
    ("io-randread", ScenarioMeta {
        display_name: "IO Random Read",
        unit: "IOPS",
        subtitle: "Tool: fio 3.36 | Test: 30s, 4KB blocks, random read",
        tooltip: "IO Random Read: Measures random read performance with 4KB blocks.\nTool: fio 3.36\nTest: 30 seconds, 4KB block size, random read\nCmd: fio --name=randread --filename=/tmp/fio.bin --size=256M --rw=randread --bs=4k --ioengine=libaio --iodepth=32 --direct=1 --runtime=10 --time_based --group_reporting",
        run_cmd: "lingbench run --scenario io-randread --vmm <vmm>",
    }),
    ("io-randwrite", ScenarioMeta {
        display_name: "IO Random Write",
        unit: "IOPS",
        subtitle: "Tool: fio 3.36 | Test: 30s, 4KB blocks, random write",
        tooltip: "IO Random Write: Measures random write performance with 4KB blocks.\nTool: fio 3.36\nTest: 30 seconds, 4KB block size, random write\nCmd: fio --name=randwrite --filename=/tmp/fio.bin --size=256M --rw=randwrite --bs=4k --ioengine=libaio --iodepth=32 --direct=1 --runtime=10 --time_based --group_reporting",
        run_cmd: "lingbench run --scenario io-randwrite --vmm <vmm>",
    }),
    ("io-seqread", ScenarioMeta {
        display_name: "IO Sequential Read",
        unit: "IOPS",
        subtitle: "Tool: fio 3.36 | Test: 30s, 4KB blocks, sequential read",
        tooltip: "IO Sequential Read: Measures sequential read performance with 4KB blocks.\nTool: fio 3.36\nTest: 30 seconds, 4KB block size, sequential read\nCmd: fio --name=seqread --filename=/tmp/fio.bin --size=256M --rw=read --bs=1M --ioengine=libaio --iodepth=16 --direct=1 --runtime=10 --time_based --group_reporting",
        run_cmd: "lingbench run --scenario io-seqread --vmm <vmm>",
    }),
    ("io-seqwrite", ScenarioMeta {
        display_name: "IO Sequential Write",
        unit: "IOPS",
        subtitle: "Tool: fio 3.36 | Test: 30s, 4KB blocks, sequential write",
        tooltip: "IO Sequential Write: Measures sequential write performance with 4KB blocks.\nTool: fio 3.36\nTest: 30 seconds, 4KB block size, sequential write\nCmd: fio --name=seqwrite --filename=/tmp/fio.bin --size=256M --rw=write --bs=1M --ioengine=libaio --iodepth=16 --direct=1 --runtime=10 --time_based --group_reporting",
        run_cmd: "lingbench run --scenario io-seqwrite --vmm <vmm>",
    }),
    ("app-redis", ScenarioMeta {
        display_name: "Redis GET",
        unit: "ops/s",
        subtitle: "Tool: redis-benchmark | Test: GET operations",
        tooltip: "Redis GET: Measures Redis GET operation throughput.\nTool: redis-benchmark\nTest: GET operations\nCmd: redis-server --daemonize yes && redis-benchmark -q -n 100000 -c 50 -P 16",
        run_cmd: "lingbench run --scenario app-redis --vmm <vmm>",
    }),
    ("app-nginx", ScenarioMeta {
        display_name: "Nginx QPS",
        unit: "req/s",
        subtitle: "Tool: wrk | Test: HTTP requests per second",
        tooltip: "Nginx QPS: Measures Nginx HTTP request throughput.\nTool: wrk\nTest: HTTP requests per second\nCmd: nginx && wrk -t2 -c64 -d10s http://127.0.0.1/",
        run_cmd: "lingbench run --scenario app-nginx --vmm <vmm>",
    }),
    ("app-memcached", ScenarioMeta {
        display_name: "Memcached QPS",
        unit: "ops/s",
        subtitle: "Tool: memaslap | Test: GET operations",
        tooltip: "Memcached QPS: Measures Memcached GET operation throughput.\nTool: masaslap\nTest: GET operations\nCmd: memcached -u nobody -d && nc -w1 127.0.0.1 11211",
        run_cmd: "lingbench run --scenario app-memcached --vmm <vmm>",
    }),
];

fn get_all_scenarios() -> Vec<String> {
    SCENARIO_META.iter().map(|(n, _)| (*n).to_string()).collect()
}

fn get_scenario_meta(name: &str) -> Option<&'static ScenarioMeta> {
    SCENARIO_META.iter().find(|(n, _)| *n == name).map(|(_, m)| m)
}

// ============ VMM COLOR MAPPING ============

fn get_vmm_color(name: &str) -> &'static str {
    match name {
        "firecracker" => "#ff7043",
        "cloud-hypervisor" => "#4fc3f7",
        "stratovirt" => "#66bb6a",
        "crosvm" => "#ba68c8",
        _ => "#888888",
    }
}

// ============ REPORT DATA BUILDING ============

fn build_report_data(results: &[RunResult]) -> ReportData {
    // Group results by VMM, maintaining insertion order
    let mut vmm_map: Vec<(String, Vec<&RunResult>)> = Vec::new();
    for result in results {
        if let Some((_, v)) = vmm_map.iter_mut().find(|(n, _)| n == &result.vmm) {
            v.push(result);
        } else {
            vmm_map.push((result.vmm.clone(), vec![result]));
        }
    }

    let vmm_results: Vec<VmmResult> = vmm_map.iter().map(|(vmm_name, results)| {
        let scenarios: Vec<ScenarioResult> = results.iter().map(|r| {
            let status = if r.success { "OK".to_string() } else { r.error.clone().unwrap_or_default() };
            ScenarioResult {
                name: r.scenario.clone(),
                metrics: r.metrics.iter().map(|m| NamedMetric {
                    name: m.name.clone(),
                    value: m.value,
                    formatted: m.formatted_value(),
                }).collect(),
                status,
            }
        }).collect();
        VmmResult {
            name: vmm_name.clone(),
            scenarios,
        }
    }).collect();

    // Build charts
    let charts = build_charts(&vmm_results);

    // Build scenario rows for the table
    let scenario_rows = build_scenario_rows(&vmm_results);

    // Generate date string
    let date = format_timestamp(0);

    ReportData {
        date,
        vmm_results,
        charts,
        scenario_rows,
    }
}

// ============ BUILD CHARTS ============

fn build_charts(vmm_results: &[VmmResult]) -> Vec<ChartData> {
    // Skip if only 1 VMM
    if vmm_results.len() < 2 {
        return vec![];
    }

    // Chart scenarios in specific order (CPU -> Memory -> IO)
    let chart_scenario_order = [
        "cpu-sysbench", "cpu-coremark", "cpu-stress",
        "mem-sysbench",
        "io-randread", "io-randwrite", "io-seqread",
        "app-redis", "app-nginx", "app-memcached"
    ];

    let mut charts = Vec::new();

    // Get all scenario names across all VMMs, respecting chart_scenario_order
    let all_scenarios: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for scenario_name in chart_scenario_order {
            for vmm in vmm_results {
                for scenario in &vmm.scenarios {
                    if scenario.name == scenario_name && !seen.contains(scenario_name) {
                        seen.insert(scenario_name.to_string());
                        result.push(scenario_name.to_string());
                    }
                }
            }
        }
        result
    };

    for scenario_name in all_scenarios {
        // Collect data points for this scenario across all VMMs
        let mut data_points: Vec<(&str, f64)> = Vec::new();
        for vmm in vmm_results {
            if let Some(scenario_result) = vmm.scenarios.iter().find(|s| s.name == scenario_name) {
                if !scenario_result.metrics.is_empty() {
                    data_points.push((vmm.name.as_str(), scenario_result.metrics[0].value));
                }
            }
        }

        if data_points.len() < 2 {
            continue;
        }

        // Build labels and values
        let labels: Vec<&str> = data_points.iter().map(|(n, _)| *n).collect();
        let values: Vec<f64> = data_points.iter().map(|(_, v)| *v).collect();

        // Calculate dynamic Y-axis
        let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let (suggested_min, suggested_max, delta_pct) = calculate_nice_y_axis(min_val, max_val);

        // Build title with Δ% if delta > 0.1%
        let meta = get_scenario_meta(&scenario_name);
        let base_title = match meta {
            Some(m) => m.display_name.to_string(),
            None => scenario_name.clone(),
        };
        let title = if delta_pct > 0.1 {
            format!("{} (Δ {:.1}%)", base_title, delta_pct)
        } else {
            base_title
        };

        // Build colors array
        let colors: Vec<&str> = data_points.iter().map(|(n, _)| get_vmm_color(n)).collect();

        // Build Y-axis config using serde_json
        let y_scale = if suggested_min <= 0.0 || min_val < suggested_max * 0.05 {
            json!({
                "beginAtZero": true,
                "grid": { "color": "rgba(255,255,255,0.05)" },
                "ticks": { "color": "#888" }
            })
        } else {
            json!({
                "beginAtZero": false,
                "suggestedMin": suggested_min,
                "suggestedMax": suggested_max,
                "grid": { "color": "rgba(255,255,255,0.05)" },
                "ticks": { "color": "#888" }
            })
        };

        // Build x-scale
        let x_scale = json!({
            "ticks": { "color": "#888" },
            "grid": { "color": "rgba(255,255,255,0.05)" }
        });

        // Build full chart config using serde_json
        let config = json!({
            "type": "bar",
            "data": {
                "labels": labels,
                "datasets": [{
                    "label": "",
                    "data": values,
                    "backgroundColor": colors
                }]
            },
            "options": {
                "responsive": true,
                "maintainAspectRatio": false,
                "plugins": {
                    "legend": { "display": false }
                },
                "scales": {
                    "x": x_scale,
                    "y": y_scale
                }
            }
        });

        let chart_id = to_camel_case(&scenario_name) + "Chart";

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, ch) in s.chars().enumerate() {
        if ch == '-' || ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            // Capitalize first letter after separator, preserve case for rest of word
            result.push_str(&ch.to_uppercase().to_string());
            capitalize_next = false;
        } else if i == 0 {
            // First char of first word is lowercase
            result.push(ch.to_ascii_lowercase());
        } else {
            // Preserve existing case
            result.push(ch);
        }
    }
    result
}
        let subtitle = meta.map(|m| m.subtitle).unwrap_or("").to_string();
        let tooltip = meta.map(|m| m.tooltip).unwrap_or("").to_string();

        charts.push(ChartData {
            id: chart_id,
            title,
            subtitle,
            tooltip,
            config: config.to_string(),
            delta_pct,
        });
    }

    charts
}

// ============ BUILD SCENARIO ROWS ============

fn build_scenario_rows(vmm_results: &[VmmResult]) -> Vec<ScenarioRow> {
    // Use SCENARIO_META order for scenario names
    let scenario_names: Vec<String> = SCENARIO_META.iter()
        .map(|(name, _)| name.to_string())
        .collect();

    scenario_names.into_iter().map(|name| {
        let meta = get_scenario_meta(&name);
        let display_name = match meta {
            Some(m) => format!("{} ({})", m.display_name, m.unit),
            None => name.clone(),
        };

        let mut cells: Vec<String> = Vec::new();
        for vmm in vmm_results {
            if let Some(scenario) = vmm.scenarios.iter().find(|s| s.name == name) {
                if scenario.metrics.is_empty() {
                    cells.push(scenario.status.clone());
                } else {
                    // Format multiple metrics
                    let cell = scenario.metrics.iter()
                        .map(|m| m.formatted.clone())
                        .collect::<Vec<_>>()
                        .join(", ");
                    cells.push(cell);
                }
            } else {
                cells.push("-".to_string());
            }
        }

        ScenarioRow {
            name: name.clone(),
            display_name,
            cells,
        }
    }).collect()
}

// ============ Y-AXIS CALCULATION ============

/// Calculate nice Y-axis bounds (round to nearest 100/1000/etc)
/// Returns (suggested_min, suggested_max, delta_percentage)
fn calculate_nice_y_axis(min_val: f64, max_val: f64) -> (f64, f64, f64) {
    let data_diff = max_val - min_val;

    if data_diff <= 0.0 {
        return (0.0, 100.0, 0.0);
    }

    // Calculate delta percentage
    let delta_pct = if min_val > 0.0 {
        ((max_val - min_val) / min_val) * 100.0
    } else {
        0.0
    };

    // Add 20% padding
    let padding = data_diff * 0.2;
    let raw_min = min_val - padding;
    let raw_max = max_val + padding;

    // Determine the "nice" step based on range
    let range = raw_max - raw_min;
    let nice_step = if range <= 200.0 {
        100.0
    } else if range <= 2000.0 {
        100.0
    } else if range <= 20000.0 {
        1000.0
    } else if range <= 200000.0 {
        10000.0
    } else if range <= 2000000.0 {
        100000.0
    } else {
        1000000.0
    };

    let nice_min = (raw_min / nice_step).floor() * nice_step;
    let nice_max = (raw_max / nice_step).ceil() * nice_step;

    (nice_min, nice_max, delta_pct)
}

// ============ UTILITIES ============

fn format_timestamp(_secs: u64) -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M").to_string()
}
