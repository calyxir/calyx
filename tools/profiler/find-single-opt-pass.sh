if [ $# -ne 2 ]; then
    
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
TMP_DIR=${SCRIPT_DIR}/data/single-opt
mkdir -p ${TMP_DIR}
