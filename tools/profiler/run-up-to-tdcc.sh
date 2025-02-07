# small script to get the TDCC-translated Calyx program before it goes through any other passes

if [ $# -lt 1 ]; then
    echo "USAGE: bash $0 CALYX_FILE [OPT]"
    echo "if OPT is not -no, then runs with all optimizations (in august 2024) enabled"
    exit
fi

SCRIPT_DIR=$( cd $( dirname $0 ) && pwd )
CALYX_DIR=$( dirname $( dirname ${SCRIPT_DIR} ) )

if [ "$2" == "-no" ]; then
    (
    cd ${CALYX_DIR}
    cargo run $1 -p compile-repeat -p well-formed -p papercut -p canonicalize -p compile-sync -p simplify-with-control -p compile-invoke -p static-inline -p merge-assigns -p dead-group-removal -p simplify-static-guards -p add-guard -p static-fsm-opts -p compile-static -p dead-group-removal -p tdcc
    )
else
(
    cd ${CALYX_DIR}
    cargo run $1 -p profiler-instrumentation -p compile-repeat -p well-formed -p papercut -p canonicalize -p infer-data-path -p collapse-control -p compile-sync-without-sync-reg -p group2seq -p dead-assign-removal -p group2invoke -p infer-share -p inline -p comb-prop -p dead-cell-removal -p cell-share -p simplify-with-control -p compile-invoke -p static-inference -p static-promotion -p compile-repeat -p dead-group-removal -p collapse-control -p static-inline -p merge-assigns -p dead-group-removal -p simplify-static-guards -p add-guard -p static-fsm-opts -p compile-static -p dead-group-removal # -p tdcc
)
fi
