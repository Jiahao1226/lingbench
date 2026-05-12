//! VMM real run integration tests
//!
//! These tests actually start VM and verify:
//! 1. Whether VM can start successfully
//! 2. Whether serial log has correct output
//! 3. Whether metrics are correctly collected
//!
//! Run: cargo test --test vmm_integration_tests -- --ignored

use std::fs;
use std::process::Command;
use std::path::Path;

/// Ensure lingbench binary exists
fn ensure_binary() {
    assert!(
        Path::new("./target/release/lingbench").exists(),
        "lingbench binary not found. Run 'cargo build --release' first."
    );
}

/// Get serial log path
fn get_serial_log(vmm: &str) -> String {
    match vmm {
        "firecracker" => "/tmp/fc-serial.log".to_string(),
        "cloud-hypervisor" => "/tmp/ch-serial.log".to_string(),
        "crosvm" => "/tmp/crosvm-serial.log".to_string(),
        "stratovirt" => "/tmp/stratovirt-serial.log".to_string(),
        _ => panic!("Unknown VMM: {}", vmm),
    }
}

/// Clean up VM process and serial log
fn cleanup_vmm(vmm: &str) {
    let _ = Command::new("pkill").arg("-9").arg(vmm).output();
    let log_path = get_serial_log(vmm);
    let _ = fs::remove_file(&log_path);
}

/// Verify serial log contains successful completion marker
fn assert_successful_completion(serial_log: &str, scenario: &str) {
    assert!(
        serial_log.contains(&format!("LINGBENCH_RESULT_BEGIN {}", scenario)),
        "Should contain LINGBENCH_RESULT_BEGIN"
    );
    assert!(
        serial_log.contains("LINGBENCH_RESULT_END"),
        "Should contain LINGBENCH_RESULT_END"
    );
    assert!(
        serial_log.contains("rc=0"),
        "Should contain rc=0 (success)"
    );
}

/// Verify serial log contains sysbench metrics
fn assert_sysbench_metrics(serial_log: &str) {
    assert!(
        serial_log.contains("events per second"),
        "Should contain 'events per second'"
    );
    // Verify has value
    assert!(
        serial_log.contains("556") || serial_log.contains("557") || serial_log.contains("558") || serial_log.contains("559") || serial_log.contains("560"),
        "Should contain reasonable events per second value (5500-5600)"
    );
}

// ============================================================
// Firecracker Tests
// ============================================================

#[test]
#[ignore]
fn test_firecracker_cpu_sysbench_runs_successfully() {
    ensure_binary();
    cleanup_vmm("firecracker");
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "firecracker",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/test-firecracker",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(
        output.status.success(),
        "lingbench run should succeed"
    );
    
    // Check serial log
    let serial_log = fs::read_to_string("/tmp/fc-serial.log")
        .expect("Should have serial log");
    
    assert_successful_completion(&serial_log, "cpu-sysbench");
    assert_sysbench_metrics(&serial_log);
    
    cleanup_vmm("firecracker");
}

#[test]
#[ignore]
fn test_firecracker_generates_report_with_metrics() {
    ensure_binary();
    cleanup_vmm("firecracker");
    
    let output_dir = "/tmp/test-firecracker-report";
    let _ = fs::remove_dir_all(output_dir);
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "firecracker",
            "--scenario", "cpu-sysbench",
            "--output", output_dir,
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(output.status.success());
    
    // check if report is generated
    let report_path = format!("{}/lingbench_report.html", output_dir);
    assert!(
        Path::new(&report_path).exists(),
        "Report should be generated at {}",
        report_path
    );
    
    // check report content
    let report = fs::read_to_string(&report_path).unwrap();
    assert!(
        report.contains("events_per_second") || report.contains("events per second"),
        "Report should contain metrics"
    );
    
    cleanup_vmm("firecracker");
}

// ============================================================
// Cloud-Hypervisor Tests
// ============================================================

#[test]
#[ignore]
fn test_cloud_hypervisor_cpu_sysbench_runs_successfully() {
    ensure_binary();
    cleanup_vmm("cloud-hypervisor");
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "cloud-hypervisor",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/test-ch",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(
        output.status.success(),
        "lingbench run should succeed"
    );
    
    let serial_log = fs::read_to_string("/tmp/ch-serial.log")
        .expect("Should have serial log");
    
    assert_successful_completion(&serial_log, "cpu-sysbench");
    assert_sysbench_metrics(&serial_log);
    
    cleanup_vmm("cloud-hypervisor");
}

// ============================================================
// Crosvm Tests
// ============================================================

#[test]
#[ignore]
fn test_crosvm_cpu_sysbench_runs_successfully() {
    ensure_binary();
    cleanup_vmm("crosvm");
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "crosvm",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/test-crosvm",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(
        output.status.success(),
        "lingbench run should succeed"
    );
    
    let serial_log = fs::read_to_string("/tmp/crosvm-serial.log")
        .expect("Should have serial log");
    
    assert_successful_completion(&serial_log, "cpu-sysbench");
    assert_sysbench_metrics(&serial_log);
    
    cleanup_vmm("crosvm");
}

// ============================================================
// Stratovirt Tests
// ============================================================

#[test]
#[ignore]
fn test_stratovirt_cpu_sysbench_runs_successfully() {
    ensure_binary();
    cleanup_vmm("stratovirt");
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "stratovirt",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/test-stratovirt",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(
        output.status.success(),
        "lingbench run should succeed"
    );
    
    let serial_log = fs::read_to_string("/tmp/stratovirt-serial.log")
        .expect("Should have serial log");
    
    assert_successful_completion(&serial_log, "cpu-sysbench");
    assert_sysbench_metrics(&serial_log);
    
    cleanup_vmm("stratovirt");
}

// ============================================================
// Multi-VMM Tests
// ============================================================

#[test]
#[ignore]
fn test_all_vmm_can_run_cpu_sysbench() {
    ensure_binary();
    
    let vmm_list = vec!["firecracker", "cloud-hypervisor", "crosvm", "stratovirt"];
    
    for vmm in vmm_list {
        cleanup_vmm(vmm);
        
        let output = Command::new("./target/release/lingbench")
            .args([
                "run",
                "--vmm", vmm,
                "--scenario", "cpu-sysbench",
                "--output", &format!("/tmp/test-{}-multi", vmm),
            ])
            .output()
            .expect(&format!("Failed to run {}", vmm));
        
        assert!(
            output.status.success(),
            "{} should run successfully",
            vmm
        );
        
        // Check serial log
        let serial_path = get_serial_log(vmm);
        let serial_log = fs::read_to_string(&serial_path)
            .expect(&format!("Should have serial log for {}", vmm));
        
        assert!(
            serial_log.contains("LINGBENCH_RESULT_END"),
            "{} should complete successfully",
            vmm
        );
        
        cleanup_vmm(vmm);
    }
}

// ============================================================
// Metrics Collection Tests
// ============================================================

#[test]
#[ignore]
fn test_metrics_are_collected_from_serial_log() {
    ensure_binary();
    cleanup_vmm("firecracker");
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "firecracker",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/test-metrics",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    assert!(output.status.success());
    
    // Serial log should have metrics data
    let serial_log = fs::read_to_string("/tmp/fc-serial.log").unwrap();
    
    // Verify can parse value
    use regex::Regex;
    let re = Regex::new(r"events per second:\s*([0-9.]+)").unwrap();
    if let Some(caps) = re.captures(&serial_log) {
        let value: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
        assert!(
            value > 5000.0 && value < 6500.0,
            "events per second should be in reasonable range, got {}",
            value
        );
    } else {
        panic!("Should be able to parse events per second from serial log");
    }
    
    cleanup_vmm("firecracker");
}

// ============================================================
// Error Handling Tests
// ============================================================

#[test]
#[ignore]
fn test_invalid_vmm_reports_error() {
    ensure_binary();
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "invalid-vmm",
            "--scenario", "cpu-sysbench",
            "--output", "/tmp/test-invalid",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    // Should fail
    assert!(
        !output.status.success(),
        "Invalid VMM should report error"
    );
}

#[test]
#[ignore]
fn test_invalid_scenario_reports_error() {
    ensure_binary();
    
    let output = Command::new("./target/release/lingbench")
        .args([
            "run",
            "--vmm", "firecracker",
            "--scenario", "invalid-scenario",
            "--output", "/tmp/test-invalid",
        ])
        .output()
        .expect("Failed to run lingbench");
    
    // Should fail
    assert!(
        !output.status.success(),
        "Invalid scenario should report error"
    );
}
