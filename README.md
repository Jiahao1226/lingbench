# LingBench

A VMM (Virtual Machine Monitor) benchmarking framework for Linux. Run standardized performance tests across multiple VMMs and scenarios, with real-time progress output and auto-generated HTML reports.

## Features

- **Multi-VMM Support**: Firecracker, Cloud-Hypervisor, Crosvm, Stratovirt
- **11 Benchmark Scenarios**: CPU (sysbench, coremark, stress-ng), Memory (sysbench, meminfo), I/O (fio randread/randwrite/seqread/seqwrite), Application (Redis, Nginx, Memcached)
- **Real-time Progress**: Live terminal output showing VMM startup, scenario status, and results
- **HTML Reports**: Auto-generated reports with Chart.js bar charts comparing VMM performance (10 scenario charts + detailed results table)
- **Light Theme**: Clean, readable report design with scenario tooltips
- **Results Export**: JSON output for post-processing or report regeneration
- **Graceful Shutdown**: Ctrl+C interrupts cleanly, killing VMM instances
- **TOML Configuration**: All paths, VMMs, and settings managed via `lingbench.toml`

## Supported VMMs

| VMM | Type | Configuration |
|-----|------|---------------|
| Firecracker | API socket | `--api-sock` |
| Cloud-Hypervisor | CLI | `--api-socket` |
| Crosvm | CLI | USB device + socket |
| Stratovirt | CLI | serial socket + vsock |

## Supported Scenarios

| Scenario | Tool | Metric |
|----------|------|--------|
| `cpu-sysbench` | sysbench | events/second |
| `cpu-coremark` | CoreMark | CoreMark score |
| `cpu-stress` | stress-ng | bogo ops/second |
| `mem-sysbench` | sysbench | ops/second |
| `meminfo` | /proc/meminfo | MemTotal/MemFree (KB) |
| `io-randread` | fio | IOPS |
| `io-randwrite` | fio | IOPS |
| `io-seqread` | fio | IOPS |
| `io-seqwrite` | fio | IOPS |
| `app-redis` | redis-benchmark | SET requests/second |
| `app-nginx` | wrk | requests/second |
| `app-memcached` | memcached | curr_connections |

## Quick Start

```bash
# Build
cargo build --release

# Run all VMMs with all scenarios
./target/release/lingbench run

# Run specific VMMs
./target/release/lingbench run --vmm firecracker,cloud-hypervisor

# Run specific scenario
./target/release/lingbench run --vmm firecracker --scenario cpu-sysbench

# Generate report from saved results
./target/release/lingbench report
```

## Configuration

Edit `lingbench.toml`:

```toml
[global]
workdir = "./build"
kernel = "./build/bzImage"
rootfs = "./build/rootfs.ext4"

[[vmm]]
name = "firecracker"
binary = "/usr/local/bin/firecracker"
enabled = true

[report]
output_dir = "./lingbench_results"
```

## Output Format

```
Running benchmarks...
VMMs: ["firecracker", "cloud-hypervisor", "crosvm", "stratovirt"]
Scenarios: ["cpu-sysbench", "cpu-coremark", ...]
firecracker | cpu-sysbench | running
firecracker | cpu-sysbench | running (5s)
firecracker | cpu-sysbench | ✓ | 11752ms | events_per_second=5.6k
----------------------------------------
========================================
[VMM Finished] firecracker
========================================
Results saved to lingbench_results/results.json
Report generated: lingbench_results/lingbench_Results_2026-04-28_12-00.html
```

Report filename format: `lingbench_Results_YYYY-MM-DD_HH-MM.html`

## Report Structure

HTML report includes:
- **Header**: LingBench logo, date, environment
- **Charts**: Bar charts for 10 scenarios (CPU → Memory → IO → App ordering)
- **Detailed Results Table**: All VMMs × all scenarios with formatted values
- **Tooltips**: Hover on scenario title to see benchmark command details

## Keyboard Interrupt

Press `Ctrl+C` to gracefully stop. The tool will:
1. Print `Received Ctrl+C, cleaning up...`
2. Kill all running VMM instances
3. Exit cleanly

## Project Structure

```
lingbench/
├── Cargo.toml              # Workspace manifest
├── lingbench.toml          # Configuration
├── assets/
│   └── lingcage_logo.svg   # Report logo
├── src/
│   ├── main.rs             # CLI entry point, report orchestration
│   ├── cli.rs              # Command-line argument parsing
│   └── lib/
│       ├── lib.rs          # Library exports
│       ├── config.rs       # TOML configuration loader
│       ├── kernel.rs       # Kernel handling
│       ├── rootfs.rs       # Rootfs image management
│       ├── metrics/        # Metrics collection
│       │   ├── collector.rs   # Metric ingestion and storage
│       │   ├── protocol.rs    # HostMessage parsing
│       │   └── types.rs       # Metric types
│       ├── report/         # HTML report generation
│       │   ├── data.rs        # Report data structures
│       │   └── generator.rs   # HTML template generation
│       ├── runner/         # Benchmark orchestration
│       │   └── scenario.rs    # Scenario execution
│       └── vmm/            # VMM implementations
│           ├── traits.rs      # VmmRunner/VmInstance traits
│           ├── registry.rs    # VMM auto-detection
│           ├── firecracker.rs  # Firecracker implementation
│           ├── cloudhypervisor.rs # Cloud-Hypervisor implementation
│           ├── crosvm.rs      # Crosvm implementation
│           └── stratovirt.rs  # Stratovirt implementation
├── guest/                  # Guest environment
│   ├── rootfs/            # Rootfs build (Containerfile)
│   └── kernel/            # Kernel build
├── tests/                 # Integration tests
│   ├── cli_integration_tests.rs
│   ├── metrics_validation_tests.rs
│   └── vmm_integration_tests.rs
└── build/                 # Build output (gitignored)
```

## Dependencies

- Rust (stable, edition 2024)
- Guest kernel: bzImage at path configured in `lingbench.toml`
- Guest rootfs: ext4 image with benchmark tools (sysbench, fio, stress-ng, redis, nginx, memcached)
