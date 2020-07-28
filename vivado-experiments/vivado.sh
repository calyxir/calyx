#!/bin/sh

# -e: fail on first error.
# -u: fail on unset vars.
# -f: disable filename globbing.
set -euf

usage="$(basename $0) [-h] <futil|hls> <cpp or sv src> <dest dir>
Requires env variable: \$SERVER to be set to a server with Vivado installed.

In 'futil' mode, script copies the .sv source along with
relevant .tcl and .xdc files and runs synthesis with 'vivado'.

In 'hls' mode, script copies the .cpp source along with
relevant .tcl files and runs synthesis with 'vivado_hls'.

For both modes, the results are copied back to <dest dir>."

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

# local sources to send to server
script_dir=$(dirname "$0") # gets the directory of the script
futil_sources="$script_dir/device.xdc $script_dir/synth.tcl"
hls_sources="$script_dir/hls.tcl"

# get the server address from the environment, failing if not set
if [ -z "$SERVER" ]; then
    echo "Environment variable SERVER is not set."
    exit 1
fi
server=$SERVER

# give names to arguments
type="$1" # either 'hls' or 'futil' and decides
source_file="$2"
dest_dir="$3"

# create a temporary directory on the server
workdir=$(ssh "$server" "mktemp -d")
echo "Working in $server:$workdir"

# register cleanup function so that if something fails, we don't leave tmp dirs around
cleanup() {
    if [ $? -eq 0 ]; then
        echo "$(date) $type $source_file" >> vivado_success
    else
        echo "$(date) $type $source_file" >> vivado_fail
    fi

    ssh "$server" "rm -rf $workdir"
    echo "Cleaning up $server:$workdir"
}
trap cleanup EXIT

# run Vivado on futil sources over ssh at $SERVER
futil_vivado() {
    # copy over verilog sources + other sources
    rsync $source_file $futil_sources "$server:$workdir"
    ssh $server <<EOF
    # load vivado commands
    source /opt/Xilinx/Vivado/2019.1/settings64.sh
    # move into the directory we copied over files
    cd $workdir
    # run the vivado script
    vivado -mode batch -source synth.tcl
EOF

    # copy back files
    rsync -r "$server:$workdir/$synth_result_dir/" "$dest_dir/"
}

# run Vivado on futil sources over ssh at $SERVER
hls_vivado() {
    # copy over verilog sources + other sources
    rsync $source_file $hls_sources "$server:$workdir"
    ssh $server <<EOF
    # load vivado command
    source /opt/Xilinx/Vivado/2019.1/settings64.sh
    # move into the directory we copied over files
    cd $workdir
    # run the vivado script
    vivado_hls -f hls.tcl
EOF

    # copy back files
    rsync -r "$server:$workdir/$synth_result_dir/" "$dest_dir/"
}

# actually run things
if [ "$type" = "futil" ]; then
    synth_result_dir="out"
    futil_vivado
elif [ "$type" = "hls" ]; then
    synth_result_dir="benchmark.prj"
    hls_vivado
else
    echo "First argument must be either 'futil' or 'hls'."
    exit 1
fi
