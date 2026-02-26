# Utility script for fud2 to produce flame graphs from produced .folded files

if [ $# -lt 4 ]; then
    echo "USAGE: bash $0 FLAME_GRAPH_SCRIPT DATA_DIR REPR_IN REPR_OUT" # the last two arguments are mainly for fud2's need for a defined input and output.
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )

FLAME_GRAPH_SCRIPT=$1
DATA_DIR=$2
REPR_IN=$3
REPR_OUT=$4

TREES_PDF_DIR=${DATA_DIR}-png
for f in $( ls ${DATA_DIR} | grep dot$ ); do
    dot -Tpng ${DATA_DIR}/${f} > ${DATA_DIR}/${f}.png
done

for folded in $( ls ${DATA_DIR}/*.folded ); do
    base_name=$( echo "${folded}" | rev | cut -d. -f2- | rev )
    if [[ "${base_name}" == *"dahlia"* ]]; then
        # do not add extra coloring for Dahlia programs
        color_opt="--noColor;"
    else 
        color_opt=""
    fi
    if [[ "${base_name}" == *"scaled"* ]]; then
	${FLAME_GRAPH_SCRIPT} --countname="cycles" ${folded} > ${base_name}-original.svg
	python3 ${SCRIPT_DIR}/adjust-scaled-flame-svg.py ${base_name}-original.svg "${color_opt}--scale" > ${base_name}.svg
    else
        ${FLAME_GRAPH_SCRIPT} --countname="cycles" ${folded} > tmp.svg
	python3 ${SCRIPT_DIR}/adjust-scaled-flame-svg.py tmp.svg "${color_opt}--noScale" > ${base_name}.svg
	rm tmp.svg
    fi
done

${FLAME_GRAPH_SCRIPT} --countname="cycles" ${REPR_IN} > tmp.svg
python3 ${SCRIPT_DIR}/adjust-scaled-flame-svg.py tmp.svg --noScale > ${REPR_OUT}
rm tmp.svg
