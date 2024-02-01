# script to differential test verilog implementations of primitives against new firrtl implementations of primitives

if [ $# -ne 2 ]; then
    echo "USAGE: bash $0 CALYX_FILE DATA_JSON"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )

CALYX_FILE=$1
DATA_JSON=$2
if [ ! -f ${CALYX_FILE} ]; then
    echo "ERROR: Calyx file ${CALYX_FILE} does not exist!"
    exit 1
fi
if [ ! -f ${DATA_JSON} ]; then
    echo "ERROR: Data file ${DATA_JSON} does not exist!"
    exit 1
fi
FIRRTL_DIR=${SCRIPT_DIR}/firrtl
TMP_DIR=${SCRIPT_DIR}/diff-testing-tmp
rm -rf ${TMP_DIR} && mkdir -p ${TMP_DIR}
DATA_DIR=${SCRIPT_DIR}/data

VERILOG_RESULT=${TMP_DIR}/verilog-data.json
FIRRTL_RESULT=${TMP_DIR}/firrtl-data.json
rm -f ${VERILOG_RESULT} ${FIRRTL_RESULT}

function run_verilog_ver() {    # run with preexisting verilog implementation
    firrtl_file=${TMP_DIR}/toy-verilog.fir
    verilog_file=${TMP_DIR}/toy-verilog.sv
    exec_file=${SCRIPT_DIR}/verilog-exec
    log_file=${TMP_DIR}/gol-verilog
    rm -f ${exec_file}
    echo "Running verilog version... Log file: ${log_file}"
    (
        set -o xtrace
        # FIXME: I can probably do all of this under fud2 but I wanted to save intermediate data...
        # generate the firrtl
        fud2 ${CALYX_FILE} --to firrtl -o ${firrtl_file}
        # run firrtl compiler to get the verilog
        ${FIRRTL_DIR}/utils/bin/firrtl -i ${firrtl_file} -o ${verilog_file} -X sverilog
        # run icarus
        iverilog -g2012 -o ${exec_file} ${SCRIPT_DIR}/tb.sv ${verilog_file} ${SCRIPT_DIR}/primitives-for-firrtl.sv
        timeout 1m vvp ${exec_file}
        # get result
        python3 ${SCRIPT_DIR}/json-dat.py --to-json ${VERILOG_RESULT} ${DATA_DIR}
        rm ${DATA_DIR}/mem.out  # cleanup
        set +o xtrace
    ) &> ${log_file}
    # FIXME: The below doesn't work?
    if [[ $? -ne 0 ]]; then
        echo "[Extmodule] Compilation/execution failed... exiting"
        exit 1
    fi
}

function run_firrtl_ver() {     #  run with firrtl-defined primitives
    firrtl_file=${TMP_DIR}/toy-firrtl.fir
    verilog_file=${TMP_DIR}/toy-firrtl.sv
    exec_file=${SCRIPT_DIR}/firrtl-exec
    log_file=${TMP_DIR}/gol-firrtl
    rm -f ${exec_file}
    echo "Running firrtl version... Log file: ${log_file}"
    (
        set -o xtrace
        # generate the firrtl
        fud2 ${CALYX_FILE} --to firrtl --through firrtl-with-primitives -o ${firrtl_file}
        sed -i 's/output mem_clk: UInt<1>/output mem_clk: Clock/g' ${firrtl_file} # FIXME: Replace @external with ref instead of this
        # run firrtl compiler to get the verilog
        ${FIRRTL_DIR}/utils/bin/firrtl -i ${firrtl_file} -o ${verilog_file} -X sverilog # --no-check-comb-loops
        # run icarus
        iverilog -g2012 -o ${exec_file} ${SCRIPT_DIR}/mem_tb.sv ${verilog_file} ${SCRIPT_DIR}/std_mem.sv
        timeout 1m vvp ${exec_file}
        # get result
        python3 ${SCRIPT_DIR}/json-dat.py --to-json ${FIRRTL_RESULT} ${DATA_DIR}
        rm ${DATA_DIR}/mem.out
        set +o xtrace
    ) &> ${log_file}
    # FIXME: The below doesn't work?
    if [[ $? -ne 0 ]]; then
        echo "[FIRRTL Primitives] Compilation/execution failed... exiting"
        exit 1
    fi
}

function compare_results() {
    if [[ ! -f ${FIRRTL_RESULT} || ! -f ${VERILOG_RESULT} ]]; then
        echo "ERROR: At least one run failed! Exiting..."
        exit 1
    fi
    if cmp -s "${FIRRTL_RESULT}" "${VERILOG_RESULT}"; then
        echo "Success! Below is the memory output:"
        cat ${FIRRTL_RESULT}
        echo
    else
        echo "Failed! Below is the diff:"
        diff ${FIRRTL_RESULT} ${VERILOG_RESULT}
    fi
}

function setup() {
    # setup data file
    mkdir -p ${DATA_DIR}
    rm ${DATA_DIR}/*
    python3 ${SCRIPT_DIR}/json-dat.py --from-json ${DATA_JSON} ${DATA_DIR}
}

function main() {
    setup
    echo "Running differential testing with calyx file ${CALYX_FILE} and data file ${DATA_JSON}..."
    run_verilog_ver
    run_firrtl_ver
    compare_results
}

main
