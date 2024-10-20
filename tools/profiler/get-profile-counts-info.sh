# Wrapper script for running TDCC, running simulation, obtaining cycle counts information, and producing flame graphs to visualize

if [ $# -lt 2 ]; then
    echo "USAGE: bash $0 INPUT_FILE SIM_DATA_JSON [OUT_CSV]"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
SCRIPT_NAME=$( echo "$0" | rev | cut -d/ -f1 | rev )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )

INPUT_FILE=$1
SIM_DATA_JSON=$2
name=$( echo "${INPUT_FILE}" | rev | cut -d/ -f1 | rev | cut -d. -f1 )
DATA_DIR=${SCRIPT_DIR}/data/${name}
TMP_DIR=${DATA_DIR}/generated-data
if [ $# -ge 3 ]; then
    OUT_CSV=$3
else
    OUT_CSV=${TMP_DIR}/summary.csv
fi

FLAMEGRAPH_DIR=${SCRIPT_DIR}/fg-tmp

if [ ! -d ${FLAMEGRAPH_DIR} ]; then
    (
	cd ${SCRIPT_DIR}
	git clone git@github.com:brendangregg/FlameGraph.git fg-tmp
    )
fi

TMP_VERILOG=${TMP_DIR}/no-opt-verilog.sv
FSM_JSON=${TMP_DIR}/fsm.json
CELLS_JSON=${TMP_DIR}/cells.json
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

CALYX_ARGS=" -p static-inline -p compile-static -p compile-repeat -p par-to-seq -p no-opt "


# Run TDCC to get the FSM info
echo "[${SCRIPT_NAME}] Obtaining FSM info from TDCC"
(
    cd ${CALYX_DIR}
    set -o xtrace
    cargo run -- ${INPUT_FILE} ${CALYX_ARGS} -x tdcc:dump-fsm-json="${FSM_JSON}"
    set +o xtrace
) &> ${LOGS_DIR}/gol-tdcc

if [ ! -f ${FSM_JSON} ]; then
    echo "[${SCRIPT_NAME}] Failed to generate ${FSM_JSON}! Exiting"
    exit 1
fi

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
    python3 ${SCRIPT_DIR}/parse-vcd.py ${VCD_FILE} ${FSM_JSON} ${CELLS_JSON} ${OUT_CSV} ${OUT_JSON}
    set +o xtrace
) &> ${LOGS_DIR}/gol-process

if [ "$4" == "-d" ]; then
    cat ${LOGS_DIR}/gol-process | grep -v Writing # exclude lines that show paths
else
    tail -3 ${LOGS_DIR}/gol-process | head -2 # last line is the set +o xtrace, which we don't need to show
fi

echo "[${SCRIPT_NAME}] Writing visualization files"
(
    set -o xtrace
    python3 ${SCRIPT_DIR}/create-visuals.py ${OUT_JSON} ${CELLS_JSON} ${TIMELINE_VIEW_JSON} ${FSM_TIMELINE_VIEW_JSON} ${FLAME_GRAPH_FOLDED} ${FSM_FLAME_GRAPH_FOLDED} ${FREQUENCY_FLAME_GRAPH_FOLDED} ${COMPONENTS_FOLDED} ${FSM_COMPONENTS_FOLDED}
    set +o xtrace
) &> ${LOGS_DIR}/gol-visuals

echo "[${SCRIPT_NAME}] Creating flame graph svg"
(
    set -o xtrace
    for opt in "" "--inverted" "--reverse"; do
	if [ "${opt}" == "" ]; then
	    filename=flame
	else
	    filename=flame"${opt:1}"
	fi
	${FLAMEGRAPH_DIR}/flamegraph.pl ${opt} --countname="cycles" ${FLAME_GRAPH_FOLDED} > ${TMP_DIR}/${filename}.svg
	echo
	${FLAMEGRAPH_DIR}/flamegraph.pl ${opt} --countname="cycles" ${FSM_FLAME_GRAPH_FOLDED} > ${TMP_DIR}/fsm-${filename}.svg
	echo
	${FLAMEGRAPH_DIR}/flamegraph.pl ${opt} --countname="times active" ${FREQUENCY_FLAME_GRAPH_FOLDED} > ${TMP_DIR}/frequency-${filename}.svg
	echo
	${FLAMEGRAPH_DIR}/flamegraph.pl ${opt} --countname="times active" ${COMPONENTS_FOLDED} > ${TMP_DIR}/components-${filename}.svg
	echo
	${FLAMEGRAPH_DIR}/flamegraph.pl ${opt} --countname="times active" ${FSM_COMPONENTS_FOLDED} > ${TMP_DIR}/fsm-components-${filename}.svg
    done
    set +o xtrace
) &> ${LOGS_DIR}/gol-flamegraph
