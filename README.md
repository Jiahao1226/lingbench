# Lingbench

A VMM (Virtual Machine Monitor) benchmarking tool for Linux. Run standardized performance tests across multiple VMMs and scenarios, with real-time progress output and HTML report generation.

## Features

- **Multi-VMM Support**: Firecracker, Cloud-Hypervisor, Crosvm, Stratovirt
- **11 Benchmark Scenarios**: CPU (sysbench, coremark, stress-ng), Memory (sysbench, meminfo), I/O (fio randread/randwrite/seqread), Application (Redis, Nginx, Memcached)
- **Real-time Progress**: Live terminal output showing VMM startup, scenario status, and results
- **HTML Reports**: Auto-generated reports with bar charts comparing VMM performance
- **Results Export**: JSON output for post-processing or report regeneration
- **Graceful Shutdown**: Ctrl+C interrupts cleanly, killing VMM instances
- **TOML Configuration**: All paths, VMMs, and settings managed via `lingbench.toml`

## Supported VMMs

| VMM | Binary Path |
|-----|-------------|
| Firecracker | Configured in `lingbench.toml` |
| Cloud-Hypervisor | Configured in `lingbench.toml` |
| Crosvm | Configured in `lingbench.toml` |
| Stratovirt | Configured in `lingbench.toml` |

## Supported Scenarios

| Scenario | Description | Metric |
|----------|-------------|--------|
| `cpu-sysbench` | CPU performance via sysbench | events/second |
| `cpu-coremark` | CPU performance via CoreMark | score |
| `cpu-stress` | CPU stress via stress-ng | bogo ops/second |
| `mem-sysbench` | Memory performance via sysbench | ops/second |
| `meminfo` | Memory info from /proc/meminfo | MemTotal/MemFree (KB) |
| `io-randread` | Random read I/O via fio | IOPS |
| `io-randwrite` | Random write I/O via fio | IOPS |
| `io-seqread` | Sequential read I/O via fio | IOPS |
| `app-redis` | Redis GET/SET ops via redis-benchmark | requests/second |
| `app-nginx` | Nginx requests via wrk | requests/second |
| `app-memcached` | Memcached connections | curr_connections |

## Installation

```bash
# Build from source
cd /home/jiahao/vmm-benchmark/lingbench
cargo build --release

# Install chart.js (required for HTML reports)
curl -s "https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js" \
  -o ~/lingbench_results/chart.js
cp ~/lingbench_results/chart.js target/release/
cp ~/lingbench_results/chart.js target/debug/
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
binary = "/usr/bin/firecracker"
enabled = true

[report]
enabled = true
output_dir = "/home/jiahao/lingbench_results"

[logs]
enabled = true
dir = "/home/jiahao/lingbench_results/logs"
```

## Usage

```bash
# Run all VMMs with all scenarios
cargo run -- run

# Run specific VMMs (comma-separated)
cargo run -- run --vmm firecracker,cloud-hypervisor

# Run specific scenarios
cargo run -- run --vmm firecracker --scenario cpu-sysbench

# Run with custom output directory
cargo run -- run --vmm firecracker --output /path/to/output

# Generate report from saved results
cargo run -- report --input /path/to/results.json --output /path/to/output/
```

## Output Format

During a run, output looks like:

```
========================================
[VMM Started] firecracker
========================================
firecracker | cpu-sysbench | running
firecracker | cpu-sysbench | running (5s)
firecracker | cpu-sysbench | ✓ | 11752ms | events_per_second=5.58k
----------------------------------------
...
========================================
[VMM Finished] firecracker
========================================
```

- `========================================` (`=`) separator: VMM started/failed/finished
- `----------------------------------------` (`-`) separator: Scenario result (above the line)
- Result line: `vmm | scenario | ✓/✗ | duration | metric=value`

## Report Format

HTML report includes:
- **Header**: Date, environment info
- **Charts**: Bar charts comparing VMMs per scenario (only when 2+ VMMs tested)
- **Detailed Results Table**: Per-VMM, per-scenario results with status and metrics

Charts are only generated when:
- 2 or more VMMs are tested
- The specific scenario was run (no chart if only `cpu-sysbench` was run)

## Keyboard Interrupt

Press `Ctrl+C` to gracefully stop the benchmark. The tool will:
1. Print `Received Ctrl+C, cleaning up...`
2. Kill all running VMM instances
3. Exit cleanly

## File Structure

```
lingbench/
├── lingbench.toml       # Configuration file
├── src/
│   ├── main.rs         # Entry point, CLI handling, report generation
│   └── lib/
│       ├── runner/     # Scenario execution, progress output
│       ├── vmm/        # VMM-specific implementations (Firecracker, CH, Crosvm, Stratovirt)
│       ├── config.rs   # TOML config loading
│       ├── metrics.rs  # Metric collection
│       └── report/     # HTML report generation
└── build/              # Build output (kernel, rootfs, VMM binaries)
```

## Dependencies

- Rust (latest stable)
- Guest kernel: bzImage at path configured in `lingbench.toml`
- Guest rootfs: ext4 image with benchmark tools (sysbench, fio, stress-ng, redis, nginx, memcached)
