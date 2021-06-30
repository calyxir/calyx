#!/bin/bash
# Script to run Calyx program with different flag configurations.
# Usage:
#   ./flag-compare.sh <calyx program> <data>

set -euf -o pipefail

flag1='-p all -d static-timing'
flag2='-p validate -p infer-static-timing -p collapse-control -p register-unsharing -p compile-invoke -p compile -p post-opt -p lower -d static-timing'
file=$1
data=$2

echo "Running with flags: $flag1"
fud e "$file" --to dat \
  -s verilog.data "$data" \
  -s verilog.cycle_limit 1000 \
  -s futil.flags "$flag1" > out1.json &
EXEC1=$!

echo "Running with flags: $flag2"
fud e "$file" --to dat \
  -s verilog.data "$data" \
  -s verilog.cycle_limit 1000 \
  -s futil.flags "$flag2" > out2.json &
EXEC2=$!

wait $EXEC1
wait $EXEC2

echo "=========================="
echo "Diff output between files"
diff out1.json out2.json
echo "=========================="
