#!/bin/sh
# SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
#
# SPDX-License-Identifier: Apache-2.0

# Emit a boot-complete marker to the serial console. The host-side
# runner watches ttyS0 for this exact line and timestamps it to derive
# guest-ready latency (boot time).
ts=$(date +%s.%N 2>/dev/null || date +%s)
line="LINGBENCH_BOOT_COMPLETE $ts"
printf '%s\n' "$line" > /dev/ttyS0 2>/dev/null || true
printf '%s\n' "$line"
