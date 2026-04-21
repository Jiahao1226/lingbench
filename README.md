# SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
<div align="center">
    <h1>LingBench</h1>
    <p><strong>LingBench is a VMM benchmarking framework that builds a
    minimal Linux guest and drives it through a fixed suite of
    workloads. It gives apples-to-apples numbers for virtual machine
    monitors across CPU, memory, block I/O, network, and full
    application paths.</strong></p>
</div>

> **Note:** LingBench is still under heavy development. APIs, config
> schema, and artifact layout may change significantly.

## Repository layout

```
.
├── Cargo.toml
├── lingbench.toml                     # default build configuration
├── rust-toolchain.toml
├── guest/
│   ├── kernel/
│   │   └── microvm.config             # kconfig fragment merged with defconfig
│   └── rootfs/
│       ├── Containerfile              # Alpine guest image
│       └── lingbench/                 # in-guest dispatcher scripts
│           ├── boot-marker.sh
│           ├── oneshot.sh
│           └── run.sh
└── src/
    ├── main.rs                        # `lingbench` CLI
    ├── cli.rs
    └── lib/                           # library modules
        ├── lib.rs
        ├── config.rs
        ├── kernel.rs
        ├── rootfs.rs
        └── util.rs
```

## Test suites

The guest ships a fixed set of benchmarks, each chosen to exercise a
specific axis of the VMM. CoreMark is built from source in a builder
stage of the Containerfile; everything else is pulled from Alpine's apk
repositories.

- **CoreMark** — EEMBC's reference single-thread CPU benchmark. Used as
  a clean CPU-only number with no I/O in the loop.
- **sysbench** — CPU, memory, and OLTP microbenchmarks. Quick CPU/memory
  numbers and the driver for in-guest database workloads.
- **stress-ng** — pathological CPU, memory, and syscall stressors.
  Pushes the vCPU and guest kernel into corner cases the other
  benchmarks avoid.
- **fio** — block I/O benchmark. Measures virtio-blk (and
  vhost-user-blk) throughput, IOPS, and latency across read/write mixes
  and queue depths.
- **iperf3** — virtio-net TCP and UDP throughput. The primary
  network-path number.
- **wrk + nginx** — HTTP load generator driving the in-guest `nginx`.
  Exercises the combined network + syscall + userspace path under a real
  request/response workload.
- **redis** (with `redis-benchmark`) — mixed CPU + network + syscall
  workload. Stresses the small-request path that trips up schedulers and
  interrupt delivery.
- **memcached** — a second net + syscall app workload, complementary to
  redis, used to cross-check network-stack behaviour.
- **pgbench** (from `postgresql-client`) — transactional database
  workload. Exercises fsync, block I/O, and CPU together.

## Build dependencies

To build locally you need, on the host:

- **Rust** — stable toolchain, pinned via
  [rust-toolchain.toml](rust-toolchain.toml). Install via
  [rustup](https://rustup.rs):

  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **A C toolchain and kernel build prerequisites** — `build-essential`
  (or equivalent), `make`, `bc`, `bison`, `flex`, `libssl-dev`,
  `libelf-dev`, `cpio`, `tar`, `xz-utils`.

- **Podman** (default) or Docker — required to build the rootfs from
  [guest/rootfs/Containerfile](guest/rootfs/Containerfile). Override via
  the `rootfs.builder` key in [lingbench.toml](lingbench.toml).

- **e2fsprogs** — `mkfs.ext4` is used to produce the ext4 image
  (`mkfs.ext4 -d` requires a reasonably recent version).

- **cpio** — only needed when the `cpio` rootfs format is requested.

On DEB distros:

```sh
sudo apt-get update && sudo apt-get install -y \
    build-essential make bc bison flex libssl-dev libelf-dev \
    cpio tar xz-utils e2fsprogs podman
```

## Building

```sh
cargo build --release
./target/release/lingbench build all
```

The CLI surface:

```text
VMM test framework

Usage: lingbench [OPTIONS] <COMMAND>

Commands:
  build  Build guest artifacts
  clean  Remove the working directory
  help   Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  [default: lingbench.toml]
  -h, --help             Print help
  -V, --version          Print version
```

`lingbench.toml` is read from the current directory; pass `--config` to
point at a different file.

## Artifacts

All outputs land under the `workdir` from
[lingbench.toml](lingbench.toml) (`build/` by default):

- `build/downloads/linux-<version>.tar.xz` — cached upstream kernel
  tarball. Reused across rebuilds; SHA-256 verified if `kernel.sha256`
  is set.
- `build/kernel/linux-<version>/` — extracted and built kernel tree.
- `build/kernel/bzImage` (x86_64) / `Image` (arm64) — the bootable
  kernel image the VMM loads. This is the file you point `-kernel` at.
- `build/rootfs/rootfs.tar` — flat tar export of the built container,
  used as the staging source for the other formats.
- `build/rootfs/rootfs.ext4` — ext4 disk image (default format), sized
  per `rootfs.size_mib`. Attach as a virtio-blk device and boot with
  `root=/dev/vda`.
- `build/rootfs/rootfs.cpio` — newc cpio archive suitable for use as an
  initramfs (`-initrd`) when the VMM prefers that path.

Inside the guest, `/lingbench/boot-marker.sh` emits an early serial
marker on `ttyS0` for boottime measurement, `/lingbench/oneshot.sh` runs
a cmdline-driven scenario and powers off, and a respawning `getty` on
`ttyS0` is the fallback when no scenario is passed.
