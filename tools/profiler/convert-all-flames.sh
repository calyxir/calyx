# Utility script for fud2 to produce flame graphs from produced .folded files

if [ $# -lt 4 ]; then
    echo "USAGE: bash $0 FLAME_GRAPH_SCRIPT FOLDED_DIR REPR_IN"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )

FLAME_GRAPH_SCRIPT=$1
FOLDED_DIR=$2
REPR_IN=$3
REPR_OUT=$4

for folded in $( ls ${FOLDED_DIR}/*.folded ); do
    base_name=$( echo "${folded}" | rev | cut -d. -f2- | rev )
    if [[ "${base_name}" == *"scaled"* ]]; then
	${FLAME_GRAPH_SCRIPT} --countname="cycles" ${folded} > ${base_name}-original.svg
	python3 ${SCRIPT_DIR}/finagle-with-svg.py ${base_name}-original.svg > ${base_name}.svg
    else
        ${FLAME_GRAPH_SCRIPT} --countname="cycles" ${folded} > ${base_name}.svg
    fi
done

${FLAME_GRAPH_SCRIPT} ${REPR_IN} > ${REPR_OUT}
