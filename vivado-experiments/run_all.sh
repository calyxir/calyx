#!/usr/bin/env bash

## Script that runs all the benchmarks listed in the file provided
## as $1.

# -e: fail on first error.
# -u: fail on unset vars.
# -f: disable filename globbing.
set -euf

usage="$(basename $0) [-h] <benchmark list file>
Runs 'compare.sh' on every benchmark in the provided file and stores
the results in 'results/<date>'"

while getopts 'h' option; do
    case "$option" in
        h) echo "$usage"
           exit
           ;;
        \?) printf "illegal option: -%s\n" "$OPTARG" >&2
            echo "$usage" >&2
            exit 1
            ;;
    esac
done

# setup variables
script_dir=$(dirname "$0")
benchmark_file="$1"

# create directory to store the results
result_dir="results/$(date +%Y_%m_%d_%H_%M)"
mkdir -p $result_dir

# run all the benchmarks in parallel
cat $benchmark_file | parallel --bar ${*:2} "$script_dir/compare.sh -d {} {/.} $result_dir/{/.}"
