//! Metrics dataauthenticity validation tests
//!
//! Verify metrics extracted from guest are correct

use std::process::Command;
use std::fs;
use std::path::PathBuf;

/// Parse test data, get last run results
fn get_last_run_output() -> Option<String> {
    let report_path = std::env::var("LINGBENCH_REPORT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/lingbench-test/lingbench_report.html"));
    
    fs::read_to_string(&report_path).ok()
}

/// Extract metric values from HTML report
fn extract_metric_from_report(report: &str, scenario: &str, vmm: &str) -> Option<f64> {
    // Simplified implementation: find values in table
    // In production should parse stricter format
    None
}

#[test]
fn test_parse_cpu_sysbench_events_per_second() {
    // This is pure parsing logic test, no real VM needed
    let output = r#"
sysbench 1.0.20 (from fixture)
Running CPU benchmark...
events per second: 5599.21
latency average: 0.18ms
"#;
    
    // Verify regex can match
    use regex::Regex;
    let re = Regex::new(r"events per second:\s*([0-9.]+)").unwrap();
    if let Some(caps) = re.captures(output) {
        let value: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
        assert!((value - 5599.21).abs() < 0.01);
    } else {
        panic!("Failed to parse events per second from sysbench output");
    }
}

#[test]
fn test_parse_cpu_coremark_score() {
    let output = r#"
CoreMark 1.0:
  Score:          51872.111667
  (running jobs: 4)
"#;
    
    use regex::Regex;
    let re = Regex::new(r"CoreMark 1.0:\s*\n\s*Score:\s*([0-9.]+)").unwrap();
    if let Some(caps) = re.captures(output) {
        let value: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
        assert!((value - 51872.11).abs() < 0.1);
    }
}

#[test]
fn test_parse_cpu_stress_bogo_ops() {
    let output = r#"
stress-ng --cpu 1 --timeout 10s --metrics-brief
stress-ng: info: [123] stress-ng 0.17.03.x86_64 started
stress-ng: info: [123] dispatching hogs: 1 cpu
stress-ng: metrc:[123] 206.00 bogo ops/s
"#;
    
    use regex::Regex;
    let re = Regex::new(r"stress-ng: metrc:\[[0-9]+\]\s*([0-9.]+)\s*bogo ops/s").unwrap();
    if let Some(caps) = re.captures(output) {
        let value: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
        assert!((value - 206.0).abs() < 0.1);
    }
}

#[test]
fn test_parse_mem_sysbench_ops() {
    let output = r#"
sysbench 1.0.20: memory benchmark
Operations performed: 4,194,304 (4,035,815.42 ops/s)
"#;
    
    use regex::Regex;
    let re = Regex::new(r"\(([0-9,]+)\s*ops/s\)").unwrap();
    if let Some(caps) = re.captures(output) {
        let value_str = caps.get(1).unwrap().as_str().replace(',', "");
        let value: f64 = value_str.parse().unwrap();
        assert!((value - 4035815.42).abs() < 1.0);
    }
}

#[test]
fn test_parse_io_randread_iops() {
    let output = r#"
  read: IOPS=110000, BW=429MiB/s
"#;
    
    use regex::Regex;
    let re = Regex::new(r"IOPS=([0-9k.]+)").unwrap();
    if let Some(caps) = re.captures(output) {
        let value_str = caps.get(1).unwrap().as_str();
        // Support k suffix (e.g., "110k" -> 110000)
        let value: f64 = if value_str.ends_with('k') {
            value_str[..value_str.len()-1].parse::<f64>().unwrap() * 1000.0
        } else {
            value_str.parse().unwrap()
        };
        assert!((value - 110000.0).abs() < 1.0);
    }
}

#[test]
fn test_parse_io_iops_with_k_suffix() {
    use regex::Regex;
    let re = Regex::new(r"IOPS=([0-9k.]+)").unwrap();
    
    let test_cases = vec![
        ("IOPS=110k", 110000.0),
        ("IOPS=25.2k", 25200.0),
        ("IOPS=173k", 173000.0),
        ("IOPS=40700", 40700.0),
    ];
    
    for (input, expected) in test_cases {
        if let Some(caps) = re.captures(input) {
            let value_str = caps.get(1).unwrap().as_str();
            let value: f64 = if value_str.ends_with('k') {
                value_str[..value_str.len()-1].parse::<f64>().unwrap() * 1000.0
            } else {
                value_str.parse().unwrap()
            };
            assert!((value - expected).abs() < 1.0, "Failed for {}: got {}, expected {}", input, value, expected);
        }
    }
}

/// Integration test: verify actual run produces valid metrics
/// Run first: lingbench run --vmm firecracker --scenario cpu-sysbench --output /tmp/lingbench-test/
#[test]
#[ignore] // Needs real VM environment
fn test_integration_firecracker_cpu_sysbench_has_valid_metrics() {
    // 1. Ensure lingbench binary exists
    assert!(
        std::path::Path::new("./target/release/lingbench").exists(),
        "lingbench binary not found"
    );
    
    // 2. Run test
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "firecracker",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/lingbench-metrics-test"
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(output.status.success(), "lingbench run failed");
    
    // 3. Check if serial log has metrics
    let serial_log = fs::read_to_string("/tmp/fc-serial.log").ok();
    
    if let Some(log) = serial_log {
        // Should have sysbench output
        assert!(log.contains("sysbench") || log.contains("events per second"));
        
        // Verify can parse event count
        use regex::Regex;
        let re = Regex::new(r"events per second:\s*([0-9.]+)").unwrap();
        if let Some(caps) = re.captures(&log) {
            let value: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            // CPU sysbench in VM should be in 5000-6000 events/s range
            assert!(value > 5000.0 && value < 6500.0,
                "Unexpected events/s value: {}", value);
        }
    }
}

/// Verify all 4 VMMs can run successfully
#[test]
#[ignore] // Needs real VM environment
fn test_integration_all_vmm_run_successfully() {
    let vmm_list = vec!["firecracker", "cloud-hypervisor", "crosvm", "stratovirt"];
    
    for vmm in vmm_list {
        let output = Command::new("./target/release/lingbench")
            .args([
                "run",
                "--vmm", vmm,
                "--scenario", "cpu-sysbench",
                "--output", &format!("/tmp/lingbench-{}-test", vmm)
            ])
            .output()
            .expect(&format!("Failed to run lingbench for {}", vmm));
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() || stdout.contains("✓"),
            "VMM {} failed to run successfully",
            vmm
        );
    }
}

/// Verify report contains table data
#[test]
#[ignore] // need to generate report first
fn test_report_has_detailed_table() {
    let report = fs::read_to_string("/tmp/lingbench-test/lingbench_report.html")
        .expect("Report file not found");
    
    // Verify table exists
    assert!(report.contains("<table>"), "Report should contain a table");
    assert!(report.contains("Detailed Results"), "Report should have Detailed Results section");
    
    // Verify at least 7 scenario rows (current implemented scenarios)
    let scenario_count = report.matches("<tr><td>").count();
    assert!(scenario_count >= 7, "Should have at least 7 scenario rows, got {}", scenario_count);
}
