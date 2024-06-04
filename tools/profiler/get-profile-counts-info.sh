# Wrapper script for running TDCC, running simulation, and obtaining cycle counts information

if [ $# -ne 2 ]; then
    echo "USAGE: bash $0 INPUT_FILE SIM_DATA_JSON"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
SCRIPT_NAME=$( echo "$0" | rev | cut -d/ -f1 | rev )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )
TMP_DIR=${SCRIPT_DIR}/tmp
TMP_VERILOG=${TMP_DIR}/no-opt-verilog.sv
FSM_JSON=${TMP_DIR}/fsm.json
VCD_FILE=${TMP_DIR}/trace-info.vcd
LOGS_DIR=${SCRIPT_DIR}/logs
mkdir -p ${TMP_DIR} ${LOGS_DIR}

rm -f ${TMP_VERILOG} ${FSM_JSON}

INPUT_FILE=$1
SIM_DATA_JSON=$2

# Run TDCC to get the FSM info
echo "[${SCRIPT_NAME}] Obtaining FSM info from TDCC"
(
    cd ${CALYX_DIR}
    set -o xtrace
    cargo run -- ${INPUT_FILE} -p no-opt -x tdcc:dump-fsm-json="${FSM_JSON}"
    set +o xtrace
) &> ${LOGS_DIR}/gol-tdcc

# Run simuation to get VCD
echo "[${SCRIPT_NAME}] Obtaining VCD file via simulation"
(
    set -o xtrace
    fud2 ${INPUT_FILE} -o ${VCD_FILE} -s calyx.args='-p no-opt' -s sim.data=${SIM_DATA_JSON}
    set +o xtrace
) &> ${LOGS_DIR}/gol-vcd

# Run script to get cycle level counts
echo "[${SCRIPT_NAME}] Using FSM info and VCD file to obtain cycle level counts"
(
    python3 ${SCRIPT_DIR}/parse-vcd.py ${VCD_FILE} ${FSM_JSON}
) # &> ${LOGS_DIR}/gol-process
