if [ $# -lt 2 ]; then
    echo "USAGE: bash $0 INPUT_FILE SIM_DATA_JSON"
    exit
fi

INPUT_FILE=$1
SIM_DATA_JSON=$2

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
SCRIPT_NAME=$( echo "$0" | rev | cut -d/ -f1 | rev )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )
TMP_DIR=${SCRIPT_DIR}/inst-check-tmp
mkdir -p ${TMP_DIR}
rm -rf ${TMP_DIR}/*

INST_DIR=${TMP_DIR}/inst
BASE_DIR=${TMP_DIR}/no-inst
mkdir -p ${INST_DIR} ${BASE_DIR}

INST_FILE=${TMP_DIR}/inst.json
BASE_FILE=${TMP_DIR}/no-inst.json

# run with instrumentation
(
    cd ${CALYX_DIR}
    set -o xtrace
    fud2 ${INPUT_FILE} -o ${INST_FILE} --through verilator -s calyx.args='-p profiler-instrumentation -p all' -s sim.data=${SIM_DATA_JSON} --dir ${INST_DIR}
    set +o xtrace
) &> ${TMP_DIR}/gol-inst

# run without instrumentation
(
    cd ${CALYX_DIR}
    set -o xtrace
    fud2 ${INPUT_FILE} -o ${BASE_FILE} --through verilator -s calyx.args='-p all' -s sim.data=${SIM_DATA_JSON} --dir ${BASE_DIR}
    set +o xtrace
) &> ${TMP_DIR}/gol-no-inst

# diff the two dat files
echo "Diff... should be empty"
diff ${INST_FILE} ${BASE_FILE}
