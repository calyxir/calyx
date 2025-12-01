# This script compares cycle counts between the default Calyx compiler and experimental compiler arguments.
# Currently this script runs the queues programs using Verilator to benchmark.

if [ $# -lt 1 ]; then
    echo "USAGE: bash $0 CALYX_ARGS"
    echo "where CALYX_ARGS is a string in quotes specifying experimental calyx args"
    echo "ex) bash $0 \"-p simplify-if-comb -p dead-cell-removal -p dead-group-removal -p all\""
    exit
fi

PASSES=$1

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
SCRIPT_NAME=$( echo "$0" | rev | cut -d/ -f1 | rev )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )
OUT_DIR=${SCRIPT_DIR}/out
out=${OUT_DIR}/results.csv

if [ -d ${OUT_DIR} ]; then
    echo "****Moving old results"
    mv ${OUT_DIR} ${OUT_DIR}-`date +%Y-%m-%d-%H-%M-%S`
fi
mkdir -p ${OUT_DIR}

# rounding scheme
function round() {
    echo $(printf %.$2f $(echo "scale=$2;(((10^$2)*$1)+0.5)/(10^$2)" | bc))
};

(
    cd ${CALYX_DIR}
    echo "name,og,opt,diff,diff(%)" > ${out}
    py_args="20000 --keepgoing"
    for f in $( cat ${SCRIPT_DIR}/queues-tests.txt ); do
	name=$( basename "${f}" | cut -d. -f1 )
	header=$( echo "${f}" | cut -d. -f1 )
	echo ====${name}
	original=${OUT_DIR}/${name}-original.json
	opt=${OUT_DIR}/${name}-opt.json
	# run with -p all
	fud2 ${f} -o ${original} --to dat --through verilator -s sim.data=${header}.data -s py.args="${py_args}" -q
	# run with specified passes
	fud2 ${f} -o ${opt} --to dat --through verilator -s sim.data=${header}.data -s calyx.args="${PASSES}" -s py.args="${py_args}" -q
	# TODO: maybe have something that throws a warning message if more than just the cycles are different?
	original_num=$( grep cycles ${original} | rev | cut -d, -f2 | cut -d' ' -f1 | rev )
	opt_num=$( grep cycles ${opt} | rev | cut -d, -f2 | cut -d' ' -f1 | rev )
	diff=$( echo "${original_num} - ${opt_num}" | bc -l )
	diff_percent=$( round $( echo "(${diff} / ${original_num}) * 100" | bc -l ) 2 )
	echo "${name},${original_num},${opt_num},${diff},${diff_percent}" >> ${out}
    done
)
