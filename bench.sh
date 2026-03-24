#!/usr/bin/env bash
# bench.sh — Benchmark suite for agent-desktop
# Measures cold start, status, get apps, press, click, fill, snapshot
# Runs N iterations of each warm command, reports avg/min/max
#
# Usage: ./bench.sh [iterations]  (default: 5)

set -euo pipefail

ITERATIONS="${1:-5}"
BINARY="${CARGO_TARGET_DIR:-./target}/release/agent-desktop"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo "=================================================="
echo " agent-desktop benchmark suite"
echo " Iterations per command: $ITERATIONS"
echo " Binary: $BINARY"
echo "=================================================="
echo ""

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY. Run 'cargo build --release' first."
    exit 1
fi

# Helper: time a command using python3 perf_counter for sub-ms accuracy
# Returns milliseconds
time_cmd() {
    local cmd="$1"
    python3 -c "
import subprocess, time
start = time.perf_counter()
result = subprocess.run('$cmd', shell=True, capture_output=True)
end = time.perf_counter()
ms = (end - start) * 1000
print(f'{ms:.2f}')
exit(result.returncode)
" 2>/dev/null
}

# Helper: compute stats from a file of numbers (one per line)
compute_stats() {
    local file="$1"
    python3 -c "
import sys
vals = [float(line.strip()) for line in open('$file') if line.strip()]
if not vals:
    print('N/A N/A N/A')
    sys.exit(0)
avg = sum(vals) / len(vals)
mn = min(vals)
mx = max(vals)
print(f'{avg:.2f} {mn:.2f} {mx:.2f}')
"
}

# Helper: run N iterations of a command
bench_command() {
    local label="$1"
    local cmd="$2"
    local tmpfile
    tmpfile=$(mktemp)

    printf "${CYAN}%-20s${NC} " "$label"

    local failures=0
    for i in $(seq 1 "$ITERATIONS"); do
        local ms
        ms=$(time_cmd "$cmd" 2>/dev/null) || {
            failures=$((failures + 1))
            continue
        }
        echo "$ms" >> "$tmpfile"
    done

    if [ -s "$tmpfile" ]; then
        local stats
        stats=$(compute_stats "$tmpfile")
        local avg min max
        avg=$(echo "$stats" | awk '{print $1}')
        min=$(echo "$stats" | awk '{print $2}')
        max=$(echo "$stats" | awk '{print $3}')
        printf "avg=%7s ms  min=%7s ms  max=%7s ms" "$avg" "$min" "$max"
        if [ "$failures" -gt 0 ]; then
            printf "  ${RED}(%d failures)${NC}" "$failures"
        fi
        echo ""
    else
        printf "${RED}ALL FAILED${NC}\n"
    fi

    rm -f "$tmpfile"
}

# ============================================================
# 1. Cold start measurement
# ============================================================
echo -e "${YELLOW}=== Cold Start ===${NC}"

# Kill daemon
pkill -f agent-desktop-daemon 2>/dev/null || true
sleep 0.5

tmpfile_cold=$(mktemp)
for i in $(seq 1 "$ITERATIONS"); do
    # Kill daemon before each cold start measurement
    pkill -f agent-desktop-daemon 2>/dev/null || true
    sleep 0.5

    ms=$(time_cmd "$BINARY status" 2>/dev/null) || {
        echo "  Cold start attempt $i failed"
        continue
    }
    echo "$ms" >> "$tmpfile_cold"
done

if [ -s "$tmpfile_cold" ]; then
    stats=$(compute_stats "$tmpfile_cold")
    avg=$(echo "$stats" | awk '{print $1}')
    min=$(echo "$stats" | awk '{print $2}')
    max=$(echo "$stats" | awk '{print $3}')
    printf "${CYAN}%-20s${NC} avg=%7s ms  min=%7s ms  max=%7s ms\n" "cold start" "$avg" "$min" "$max"
else
    echo -e "${RED}Cold start: ALL FAILED${NC}"
fi
rm -f "$tmpfile_cold"

echo ""

# ============================================================
# 2. Warm daemon measurements
# ============================================================
echo -e "${YELLOW}=== Warm Daemon ===${NC}"

# Ensure daemon is running (warm it up)
$BINARY status > /dev/null 2>&1 || true
sleep 0.3

bench_command "status" "$BINARY status"
bench_command "get apps" "$BINARY get apps"
bench_command "press tab" "$BINARY press tab"
bench_command "snapshot -i" "$BINARY snapshot -i"

echo ""

# ============================================================
# 3. Target comparison
# ============================================================
echo -e "${YELLOW}=== Targets ===${NC}"
echo "  status    target: <25ms"
echo "  get apps  target: <10ms"
echo "  press     target: <12ms"
echo "  cold start target: <50ms"
echo ""

echo "=================================================="
echo " Benchmark complete"
echo "=================================================="
