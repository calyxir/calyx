# will overwrite builder.py!!! 
if [ $# -lt 2 ]; then
    echo "USAGE: bash $0 PROG DAT_FILE MODE"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )

PROG=$1
DAT_FILE=$2
if [ $# -eq 3 ]; then
    MODE=$3
else
    MODE=ALL
fi

CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )
FUD2_DIR=${CALYX_DIR}/fud2-runs
BUILDER_FILE=${CALYX_DIR}/calyx-py/calyx/builder.py
TMP_BUILDER_DIR=${SCRIPT_DIR}/builders-scratch/

function run_mode() {
    local mode=$1 # par, seq, or nested
    prog_name=$( echo "${PROG}" | rev | cut -d/ -f1 | rev | cut -d. -f1 )
    run_id=${prog_name}-${mode}
    # work dir
    fud2_dir=${CALYX_DIR}/fud2-runs/${run_id}
    mkdir -p ${fud2_dir}
    rm -rf ${fud2_dir}/*
    # set up new builder
    cp ${TMP_BUILDER_DIR}/builder-${mode}.py ${BUILDER_FILE}
    (
	cd ${CALYX_DIR}/calyx-py
	flit install
    )
    echo "[$0] Running with mode ${mode}"
    # fud2 ${PROG} -o ${SCRIPT_DIR}/${run_id}.futil -s sim.data=${DAT_FILE} -s calyx.args=\"-x tdcc:dump-fsm\"
    set -o xtrace
    fud2 ${PROG} -o ${run_id}.vcd -s sim.data=${DAT_FILE} -s calyx.args="-x tdcc:dump-fsm-json=\"${SCRIPT_DIR}/${run_id}.json\""
    set +o xtrace
}

function cleanup() {
    (
	cd ${CALYX_DIR}
	git checkout ${BUILDER_FILE}
    )
    (
	cd ${CALYX_DIR}/calyx-py
	flit install
    )
}

if [ "${MODE}" == "ALL" ]; then
    echo "[$0] Running with all options!"
    run_mode nested
    run_mode par
    run_mode seq
else
    run_mode "${MODE}"
fi

cleanup
