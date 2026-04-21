#!/bin/sh
# SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
# lingbench in-guest benchmark dispatcher.
#
# Invoked by the host-side runner (over vsock, ssh, or the serial
# console) to execute one named scenario and emit its output on stdout.
# Scenarios map 1:1 to the evaluation axes: boot, memory, CPU, I/O,
# network, and application workloads.
set -eu

# init-context PATH doesn't include /usr/local/bin, where coremark lives.
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export PATH

# When run from inittab the network stack hasn't been touched yet, so the
# loopback interface is down — any 127.0.0.1 connect returns ENETUNREACH.
# Bring it up unconditionally; harmless if already up.
ip link set lo up 2>/dev/null || true

usage() {
    cat <<'EOF'
usage: run.sh <scenario> [args...]

cpu:
  cpu-sysbench [threads]       sysbench cpu, 10s
  cpu-coremark                 CoreMark single-thread score
  cpu-stress                   stress-ng --cpu, metrics-brief

memory:
  mem-sysbench                 sysbench memory bandwidth, 4 GiB total
  meminfo                      /proc/meminfo + smaps_rollup snapshot

block I/O (virtio-blk):
  io-randread                  fio 4k randread, iodepth 32, 10s
  io-randwrite                 fio 4k randwrite, iodepth 32, 10s
  io-seqread                   fio 1M seqread, iodepth 16, 10s

network (DISABLED - requires host-side server + bridge config):
#  net-iperf3-tcp <host>        iperf3 TCP_STREAM for 10s
#  net-iperf3-udp <host>        iperf3 UDP_STREAM for 10s

application:
  app-redis                    redis-benchmark against a local redis
  app-nginx                    wrk against a local nginx
  app-memcached                memcached stats smoke test

all:
  all                          run all scenarios above in sequence (excludes network tests)


Any scenario can be prefixed with `time` externally; this script only
runs the workload and writes its native output to stdout.
EOF
}

scenario=${1:-}
shift 2>/dev/null || true

case "$scenario" in
    cpu-sysbench)
        sysbench cpu --threads="${1:-1}" --time=10 run
        ;;
    cpu-coremark)
        coremark
        ;;
    cpu-stress)
        stress-ng --cpu 1 --timeout 10s --metrics-brief
        ;;

    mem-sysbench)
        sysbench memory --memory-total-size=4G run
        ;;
    meminfo)
        cat /proc/meminfo
        printf -- '---\n'
        cat /proc/self/smaps_rollup 2>/dev/null || true
        ;;

    io-randread)
        fio --name=randread --filename=/tmp/fio.bin --size=256M \
            --rw=randread --bs=4k --ioengine=libaio --iodepth=32 \
            --direct=1 --runtime=10 --time_based --group_reporting
        ;;
    io-randwrite)
        fio --name=randwrite --filename=/tmp/fio.bin --size=256M \
            --rw=randwrite --bs=4k --ioengine=libaio --iodepth=32 \
            --direct=1 --runtime=10 --time_based --group_reporting
        ;;
    io-seqread)
        fio --name=seqread --filename=/tmp/fio.bin --size=256M \
            --rw=read --bs=1M --ioengine=libaio --iodepth=16 \
            --direct=1 --runtime=10 --time_based --group_reporting
        ;;

#    net-iperf3-tcp)
#        iperf3 -c "${1:?host ip required}" -t 10
#        ;;
#    net-iperf3-udp)
#        iperf3 -c "${1:?host ip required}" -u -b 0 -t 10
#        ;;

    app-redis)
        redis-server --daemonize yes --save '' --logfile /tmp/redis.log
        sleep 0.2
        redis-benchmark -q -n 100000 -c 50 -P 16
        redis-cli shutdown nosave 2>/dev/null || true
        ;;
    app-nginx)
        mkdir -p /run/nginx /var/lib/nginx/tmp /var/log/nginx
        nginx
        sleep 0.2
        wrk -t2 -c64 -d10s http://127.0.0.1/
        nginx -s stop 2>/dev/null || true
        ;;
    all)
        # Run all scenarios in sequence (excludes network tests)
        for scenario in cpu-sysbench cpu-coremark cpu-stress mem-sysbench meminfo io-randread io-randwrite io-seqread app-redis app-nginx app-memcached; do
            echo ""
            echo "========================================"
            echo "[LingBench] Running: $scenario"
            echo "========================================"
            sh "$0" "$scenario"
        done
        ;;

    app-memcached)
        memcached -u nobody -d -P /tmp/memcached.pid
        sleep 0.2
        printf 'stats\nquit\n' | nc -w1 127.0.0.1 11211 || true
        kill "$(cat /tmp/memcached.pid)" 2>/dev/null || true
        ;;

    ''|-h|--help|help)
        usage
        ;;
    *)
        printf 'unknown scenario: %s\n\n' "$scenario" >&2
        usage >&2
        exit 1
        ;;
esac
