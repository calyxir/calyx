#!/bin/bash
# Script to run Calyx program with different flag configurations.
# Usage:
#   ./flag-compare.sh <calyx program> <data>

set -uf -o pipefail

flag1='-p dead-group-removal -p all'
flag2='-p dead-group-removal -p all'
file=$1
data=$2

echo "$flag1" > out1.json
echo "$flag2" > out2.json

echo "Running with flags: $flag1"
fud e "$file" --to dat \
  -s verilog.data "$data" \
  -s verilog.cycle_limit 1000 \
  -s futil.flags "$flag1" >> out1.json &
EXEC1=$!

echo "Running with flags: $flag2"
fud e "$file" --to dat \
  -s verilog.data "$data" \
  -s verilog.cycle_limit 1000 \
  -s futil.flags "$flag2" >> out2.json &
EXEC2=$!

wait $EXEC1
wait $EXEC2

echo "=========================="
echo "Diff output between files"
diff out1.json out2.json
echo "=========================="
