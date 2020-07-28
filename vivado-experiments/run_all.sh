#!/usr/bin/env bash

## Script that runs all the benchmarks listed in the file provided
## as $1.

# -e: fail on first error.
# -u: fail on unset vars.
# -f: disable filename globbing.
set -euf

# setup variables
script_dir=$(dirname "$0")
benchmark_file="$1"

# create directory to store the results
result_dir="results/$(date +%Y_%m_%d_%H_%M)"
mkdir -p $result_dir

# run all the benchmarks in parallel
cat $benchmark_file | parallel --bar ${*:2} "$script_dir/compare.sh {} {/.} $result_dir/{/.}"
