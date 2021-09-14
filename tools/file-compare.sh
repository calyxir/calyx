#!/bin/bash
# Script to run Calyx program in different levels of lowering and compare the
# memory/main result.
# Usage:
#   ./file-compare.sh <calyx program> <data>

clear
set -euf -o pipefail

file=$1
# data=$2
# passes=(
#   "well-formed" "papercut" "guard-canonical" "infer-static-timing" "collapse-control" "resource-sharing"
#   "minimize-regs" "compile-invoke" "compile-empty" "static-timing" "top-down-cc" "dead-cell-removal"
#   "go-insertion" "component-interface-inserter" "hole-inliner" "clk-insertion" "reset-insertion"
#   "merge-assign"
# )
passes=(
    "well-formed" "papercut" "guard-canonical" "remove-comb-groups" "infer-static-timing" 
    "collapse-control" "resource-sharing" "minimize-regs" "compile-invoke" "compile-empty" 
    "tdcc" "dead-group-removal" "dead-cell-removal" "go-insertion" 
    "component-interface-inserter" "hole-inliner" "clk-insertion" "reset-insertion" 
    "merge-assign"
)
# Other passes:
# "compile-control" "externalize" "simplify-guards" "synthesis-papercut" "register-unsharing" "par-to-seq"
len=${#passes[@]}

echo "========================================================================"
echo "|                   Original Program Interpretation                    |"
echo "========================================================================"
cd ../interp
expect=$( cargo run 2>/dev/null -- "$file" | jq .memories )
echo "$expect"

echo "========================================================================"
echo "|                          Lowered Testing                             |"
echo "========================================================================"
rm -rf pass_seq
mkdir pass_seq
flag=""
for (( i = 0; i < len; i++ )); do
  pass="-p ${passes[i]}"
  flag+=" $pass"
  fud e "$file" -s futil.flags "$flag" --to futil-lowered > "pass_seq/$i-${passes[i]}.futil"
  echo "${passes[i]}"
  if [[ "${passes[i]}" == "tdcc" || "${passes[i]}" == "dead-cell-removal" || "${passes[i]}" == "go-insertion" || "${passes[i]}" == "component-interface-inserter" ]]; then
    echo "Warning: Simulation failed."
  else
    echo "$flag"
    res=$( cargo run -- "pass_seq/$i-${passes[i]}.futil" | jq .memories )
    diff=$( diff <(echo "$expect") <(echo "$res") )
    if [[ $diff == "" ]]; then
      echo "Same"
    else
      echo "Problem"
    fi
  fi
done

# rm -rf pass_seq

# Verilator
#   - output state of mem for single input
#     - lots of input
# echo "========================================================================"
# echo "|                         Verilator Testing                            |"
# echo "========================================================================"
# flag=""
# for (( i = 0; i < len; i++ )); do
#   pass="-p ${passes[i]}"
#   flag+=" $pass"
#   fud e "$file" --to dat \
#   -s verilog.data "$data" \
#   -s verilog.cycle_limit 1000 \
#   -s futil.flags "$flag" > out1.json &
# done
