# Wrapper script for running TDCC, running simulation, obtaining cycle counts information, and producing flame graphs to visualize

if [ $# -lt 2 ]; then
    echo "USAGE: bash $0 INPUT_FILE SIM_DATA_JSON"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
SCRIPT_NAME=$( echo "$0" | rev | cut -d/ -f1 | rev )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )

INPUT_FILE=$1
SIM_DATA_JSON=$2
name=$( echo "${INPUT_FILE}" | rev | cut -d/ -f1 | rev | cut -d. -f1 )
DATA_DIR=${SCRIPT_DIR}/new-data/${name}
TMP_DIR=${DATA_DIR}/generated-data
OUT_CSV=${TMP_DIR}/summary.csv

TMP_VERILOG=${TMP_DIR}/no-opt-verilog.sv
FSM_JSON=${TMP_DIR}/fsm.json
CELLS_JSON=${TMP_DIR}/cells.json
GROUPS_JSON=${TMP_DIR}/groups.json
OUT_DIR=${TMP_DIR}/out
rm -rf ${TREES_OUT_DIR}
OUT_JSON=${TMP_DIR}/dump.json
TIMELINE_VIEW_JSON=${TMP_DIR}/timeline.json
FSM_TIMELINE_VIEW_JSON=${TMP_DIR}/fsm-timeline.json
FLAME_GRAPH_FOLDED=${TMP_DIR}/flame.folded
FSM_FLAME_GRAPH_FOLDED=${TMP_DIR}/fsm-flame.folded
FREQUENCY_FLAME_GRAPH_FOLDED=${TMP_DIR}/frequency-flame.folded
COMPONENTS_FOLDED=${TMP_DIR}/components.folded
FSM_COMPONENTS_FOLDED=${TMP_DIR}/fsm-components.folded
VCD_FILE=${TMP_DIR}/trace.vcd
LOGS_DIR=${DATA_DIR}/logs
if [ -d ${DATA_DIR} ]; then
    rm -rf ${DATA_DIR} # clean out directory for run each time
fi
mkdir -p ${TMP_DIR} ${LOGS_DIR}
rm -f ${TMP_DIR}/* ${LOGS_DIR}/* # remove data from last run

FLAMEGRAPH_DIR=${SCRIPT_DIR}/fg-tmp

if [ ! -d ${FLAMEGRAPH_DIR} ]; then
    (
	cd ${SCRIPT_DIR}
	git clone git@github.com:brendangregg/FlameGraph.git fg-tmp
    )
fi

CALYX_ARGS=" -p static-inline -p compile-static -p compile-repeat -p compile-invoke -p profiler-instrumentation -p all"

# Run component-cells backend to get cell information
echo "[${SCRIPT_NAME}] Obtaining cell information from component-cells backend"
(
    cd ${CALYX_DIR}
    set -o xtrace
    cargo run --manifest-path tools/component_cells/Cargo.toml ${INPUT_FILE} -o ${CELLS_JSON}
) &> ${LOGS_DIR}/gol-cells

if [ ! -f ${CELLS_JSON} ]; then
    echo "[${SCRIPT_NAME}] Failed to generate ${CELLS_JSON}! Exiting"
    exit 1
fi

# Run simuation to get VCD
echo "[${SCRIPT_NAME}] Obtaining VCD file via simulation"
(
    set -o xtrace
    fud2 ${INPUT_FILE} -o ${VCD_FILE} --through verilator -s calyx.args="${CALYX_ARGS}" -s sim.data=${SIM_DATA_JSON}
    set +o xtrace
) &> ${LOGS_DIR}/gol-vcd

if [ ! -f ${VCD_FILE} ]; then
    echo "[${SCRIPT_NAME}] Failed to generate ${VCD_FILE}! Exiting"
    exit 1
fi

# Run script to get cycle level counts
echo "[${SCRIPT_NAME}] Using FSM info and VCD file to obtain cycle level counts"
(
    set -o xtrace
    python3 ${SCRIPT_DIR}/new-parse-vcd.py ${VCD_FILE} ${CELLS_JSON} ${OUT_DIR}
    set +o xtrace
) &> ${LOGS_DIR}/gol-process

# Convert all dot files to pdf
TREES_PDF_DIR=${OUT_DIR}-pdf
mkdir -p ${TREES_PDF_DIR}
for f in $( ls ${OUT_DIR} | grep dot$ ); do
    dot -Tpng ${OUT_DIR}/${f} > ${TREES_PDF_DIR}/${f}.png
done

${FLAMEGRAPH_DIR}/flamegraph.pl --countname="cycles" ${OUT_DIR}/flame.folded > ${OUT_DIR}/flame.svg
