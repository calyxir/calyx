#!/bin/sh

# -e: fail on first error.
# -u: fail on unset variable.
# -f: disable filename globbing.
set -euf

fuse_file="$1"
benchmark_name="$2"
result_dir="$3"
script_dir=$(dirname "$0")
debug=0

# temporary directory
workdir=$(mktemp -d)

# register cleanup function
cleanup() {
    # save the synth results if in debug mode
    if [ $debug -eq 1 ]; then
        mkdir -p _debug
        if [ -d "$workdir/futil" ]; then
            cp -r "$workdir/futil" "_debug/$benchmark_name/"
        fi
        if [ -d "$workdir/hls" ]; then
            cp -r "$workdir/hls" "_debug/$benchmark_name/"
        fi
    fi

    echo "Cleaning up $workdir"
    # rm -rf "$workdir"
}
trap cleanup EXIT


#### Generate the files #####
# generate vivado_hls file
dahlia $fuse_file > $workdir/"$benchmark_name.cpp"

# generate system verilog file
# HACK: temporarily remove <static="1"> annotations so futil is happy
dahlia $fuse_file -b futil --lower \
    | cargo run -- -b verilog -l "$script_dir/.." \
          > $workdir/"$benchmark_name.sv"

#### Synthesis ####
# run futil and then hls synthesis
$script_dir/vivado.sh 'futil' "$workdir/$benchmark_name.sv" "$workdir/futil"
$script_dir/vivado.sh 'hls' "$workdir/$benchmark_name.cpp" "$workdir/hls"

#### Process Results ####
# copy back the files we need
mkdir -p "$result_dir"
cp $workdir/futil/FutilBuild.runs/synth_1/main_utilization_synth.rpt "$result_dir/"
cp $workdir/hls/solution1/syn/report/kernel_csynth.rpt "$result_dir/"

# extract data into json files
$script_dir/extract.py futil "$result_dir"/main_utilization_synth.rpt \
                       > "$result_dir"/futil.json
$script_dir/extract.py hls "$result_dir"/kernel_csynth.rpt \
                       > "$result_dir"/hls.json
