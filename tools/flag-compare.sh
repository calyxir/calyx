#!/bin/bash
# Script to run Calyx program with different flag configurations.
# Usage:
#   ./flag-compare.sh <calyx program> <data> <final fud stage>

set -uf -o pipefail

flag1=' -p dead-group-removal -p all'
flag2=' -p dead-group-removal -p all'
file="$1"
data="$2"
out_stage=${3:-dat}

# Kill all children when Crtl-C is received
trap 'kill $(jobs -p); wait;' SIGINT SIGTERM

echo "$flag1" > out1.json
echo "$flag2" > out2.json

echo "Running with flags: $flag1"
fud e "$file" --to "$out_stage" \
  --through icarus-verilog \
  -s interpreter.data "$data" \
  -s verilog.data "$data" \
  -s verilog.cycle_limit 1000 \
  -s calyx.flags "$flag1" >> out1.json &
EXEC1=$!

echo "Running with flags: $flag2"
fud e "$file" --to "$out_stage" \
  --through icarus-verilog \
  -s interpreter.data "$data" \
  -s verilog.data "$data" \
  -s verilog.cycle_limit 1000 \
  -s calyx.flags "$flag2" >> out2.json &
EXEC2=$!

wait $EXEC1
wait $EXEC2

echo "=========================="
echo "Diff output between files"
diff out1.json out2.json
echo "=========================="
