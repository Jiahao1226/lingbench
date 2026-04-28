#!/bin/sh
# SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
#
# SPDX-License-Identifier: Apache-2.0

# lingbench one-shot benchmark runner.
#
# Reads `lingbench.scenario=<name>` (and optional `lingbench.args=<args>`,
# space-separated) from /proc/cmdline. If a scenario is set, runs it via
# /lingbench/run.sh, brackets the output on the serial console with
# LINGBENCH_RESULT_BEGIN / LINGBENCH_RESULT_END markers so the host-side
# runner can extract just the result, then powers the guest off.
#
# If no scenario is set, exits 0 silently and the normal getty/login
# entry in /etc/inittab takes over.
set -u

exec > /dev/ttyS0 2>&1

scenario=
args=
# shellcheck disable=SC2013
for tok in $(cat /proc/cmdline); do
    case "$tok" in
        lingbench.scenario=*) scenario=${tok#lingbench.scenario=} ;;
        lingbench.args=*)     args=${tok#lingbench.args=} ;;
    esac
done

[ -z "$scenario" ] && exit 0

printf 'LINGBENCH_RESULT_BEGIN %s\n' "$scenario"
# shellcheck disable=SC2086
/lingbench/run.sh "$scenario" $args
rc=$?
printf 'LINGBENCH_RESULT_END %s rc=%d\n' "$scenario" "$rc"

sync
# Try graceful ACPI shutdown first, then fall back to the reboot syscall
# and finally to a sysrq poweroff. One of these always works.
poweroff 2>/dev/null
sleep 1
poweroff -f 2>/dev/null
/sbin/halt -f 2>/dev/null
echo o > /proc/sysrq-trigger 2>/dev/null
