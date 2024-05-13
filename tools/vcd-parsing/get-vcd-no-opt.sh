# Generates traces from a no-opt compiled version of the input Calyx program (Maybe I can make fud2 do this?)

if [ $# -ne 3 ]; then
    echo "USAGE: bash $0 CALYX_PROGRAM OUT_VCD_FILE SIM_DATA_JSON"
    echo "All files should use absolute paths."
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )
TMP_DIR=${SCRIPT_DIR}/tmp
TMP_VERILOG=${TMP_DIR}/no-opt-verilog.sv
mkdir -p ${TMP_DIR}
rm -f ${TMP_VERILOG}            # "overwrite" existing sv file

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
