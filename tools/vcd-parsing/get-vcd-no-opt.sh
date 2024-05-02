if [ $# -ne 3 ]; then
    echo "USAGE: bash $0 INPUT_FILE VCD_FILE SIM_DATA_JSON"
    echo "All files should use absolute paths"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )
TMP_DIR=${SCRIPT_DIR}/tmp
TMP_VERILOG=${TMP_DIR}/no-opt-verilog.sv
rm -rf ${TMP_DIR} && mkdir ${TMP_DIR}

INPUT_FILE=$1
VCD_FILE=$2
SIM_DATA_JSON=$3

echo ===Creating no-opt Verilog file

(
    cd ${CALYX_DIR}
    cargo run -- ${INPUT_FILE} -p no-opt -b verilog > ${TMP_VERILOG}
)

echo ===Creating no-opt VCD

(
    fud2 ${TMP_VERILOG} --from verilog --to vcd -s sim.data=${SIM_DATA_JSON} -o ${VCD_FILE}
)
