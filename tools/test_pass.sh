#!/bin/bash
# Takes in two arguments
# - First argument: relative path to the test file
# - Second argument: pattern of interest

passes=(
  "well-formed" "papercut" "guard-canonical" "infer-static-timing" "collapse-control" "resource-sharing"
  "minimize-regs" "compile-invoke" "compile-empty" "static-timing" "top-down-cc" "dead-cell-removal"
  "go-insertion" "component-interface-inserter" "hole-inliner" "clk-insertion" "reset-insertion"
  "merge-assign"
)
# "compile-control" "externalize" "simplify-guards" "synthesis-papercut" "register-unsharing" "par-to-seq"
len=${#passes[@]}

rm -rf pass_seq
mkdir pass_seq
flag=""
for (( i = 0; i < $len; i++ )); do
  pass="-p ${passes[i]}"
  flag+=" $pass"
  fud e $1 -s futil.flags "$flag" --to futil-lowered > "pass_seq/$i-${passes[i]}.futil"
done

pattern=$2

echo "======================================Result======================================"
for file in pass_seq/*; do
  if [ -f "$file" ]; then
    name=$(echo $file | cut -d "/" -f 2)
    echo $name
    grep -n "$pattern" $file
    echo ""
  fi
done
