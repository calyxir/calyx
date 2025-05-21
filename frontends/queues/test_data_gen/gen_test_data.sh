#!/usr/bin/bash

# Generate custom-length command list to the specified output directory
if [[ $# -eq 2 ]]; then 
    num_cmds=$1
    tests_dir=$2
    # NOTE: hacky and will break when other tests are created.
    mkdir -p \
        ${tests_dir}/binheap/round_robin \
        ${tests_dir}/binheap/strict \
        ${tests_dir}/round_robin \
        ${tests_dir}/strict 
    echo "Number of commands: ${num_cmds}; output directory: ${tests_dir}"
else
    num_cmds=20000
    tests_dir="$(dirname "$0")/../tests"
fi

queue_size=16

data_gen_dir="$(dirname "$0")"


# For SDN, we use piezo mode when making the data file and use pifotree_oracle to 
# generate the expected output.

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds --no-err $queue_size \
    > ${tests_dir}/sdn_test.data
[[ $? -eq 0 ]] && echo "Generated sdn_test.data"

cat ${tests_dir}/sdn_test.data \
    | python3 ${data_gen_dir}/pifo_tree_oracle.py $num_cmds $queue_size --keepgoing \
    > ${tests_dir}/sdn_test.expect
[[ $? -eq 0 ]] && echo "Generated sdn_test.expect"


# For hierarchical queues, we drop piezo mode for data gen and use the appropriate
# oracle, which is either complex_tree_oracle pifo_tree_oracle.

for queue_kind in fifo pifo_tree complex_tree; do
    python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/${queue_kind}_test.data
    [[ $? -eq 0 ]] && echo "Generated ${queue_kind}_test.data"

    cat ${tests_dir}/${queue_kind}_test.data \
        | python3 ${data_gen_dir}/${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing \
        > ${tests_dir}/${queue_kind}_test.expect
    [[ $? -eq 0 ]] && echo "Generated ${queue_kind}_test.expect"
done


# For PIEO and PCQ, we drop piezo mode and enable ranks and readiness times for 
# data gen and use nwc_oracle to generate the expected output.

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds --nwc-en \
    > ${tests_dir}/pieo_test.data
[[ $? -eq 0 ]] && echo "Generated pieo_test.data"

cat ${tests_dir}/pieo_test.data \
    | python3 ${data_gen_dir}/nwc_oracle.py $num_cmds $queue_size --keepgoing \
    > ${tests_dir}/pieo_test.expect
[[ $? -eq 0 ]] && echo "Generated pieo_test.expect"

cp ${tests_dir}/pieo_test.data ${tests_dir}/pcq_test.data
[[ $? -eq 0 ]] && echo "Generated pcq_test.data"

cp ${tests_dir}/pieo_test.expect ${tests_dir}/pcq_test.expect
[[ $? -eq 0 ]] && echo "Generated pcq_test.expect"


# For the Binary Heap, we drop piezo mode and enable ranks for data gen and
# use binheap_oracle to generate the expected output.

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds --use-rank \
    > ${tests_dir}/binheap/stable_binheap_test.data
[[ $? -eq 0 ]] && echo "Generated binheap/stable_binheap_test.data"

cat ${tests_dir}/binheap/stable_binheap_test.data \
    | python3 ${data_gen_dir}/binheap_oracle.py $num_cmds $queue_size --keepgoing \
    > ${tests_dir}/binheap/stable_binheap_test.expect
[[ $? -eq 0 ]] && echo "Generated binheap/stable_binheap_test.expect"


# For the Round Robin queues, we drop piezo mode as well and use rr_oracle to
# generate the expected output for queues with 2..7 flows. This generates 6 data 
# expect file pairs.

for n in {2..7}; do
    python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/round_robin/rr_${n}flow_test.data
    [[ $? -eq 0 ]] && echo "Generated round_robin/rr_${n}flow_test.data"

    cat ${tests_dir}/round_robin/rr_${n}flow_test.data \
        | python3 ${data_gen_dir}/rr_oracle.py $num_cmds $queue_size $n --keepgoing \
        > ${tests_dir}/round_robin/rr_${n}flow_test.expect
    [[ $? -eq 0 ]] && echo "Generated round_robin/rr_${n}flow_test.expect"
done


# For Strict queues, we use strict_oracle to generate the expected output
# for queues with 2..7 flows, each with a different strict ordering. This generates 6
# data expect file pairs.

for n in {2..7}; do
    python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/strict/strict_${n}flow_test.data
    [[ $? -eq 0 ]] && echo "Generated strict/strict_${n}flow_test.data"

    cat ${tests_dir}/strict/strict_${n}flow_test.data \
        | python3 ${data_gen_dir}/strict_oracle.py $num_cmds $queue_size $n --keepgoing \
        > ${tests_dir}/strict/strict_${n}flow_test.expect
    [[ $? -eq 0 ]] && echo "Generated strict/strict_${n}flow_test.expect"
done

# Tests for a specific ordering different from the default
python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
    > ${tests_dir}/strict/strict_order_test.data
[[ $? -eq 0 ]] && echo "Generated strict/strict_order_test.data"

cat ${tests_dir}/strict/strict_order_test.data \
    | python3 ${data_gen_dir}/strict_oracle.py $num_cmds $queue_size 3 --keepgoing --order 2,0,1 \
    > ${tests_dir}/strict/strict_order_test.expect
[[ $? -eq 0 ]] && echo "Generated strict/strict_order_test.expect"


# Copying into binheap/ for heap-based implementations of previous queues: namely,
# - FIFO
# - Complex Tree
# - Round Robin
# - Strict

cp ${tests_dir}/fifo_test.data ${tests_dir}/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/fifo_test.data"

cp ${tests_dir}/fifo_test.expect ${tests_dir}/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/fifo_test.expect"

cp $tests_dir/complex_tree_test.data $tests_dir/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/complex_tree_test.data"

cp $tests_dir/complex_tree_test.expect $tests_dir/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/complex_tree_test.expect"

for sched_algo in round_robin strict; do 
    for i in ${tests_dir}/${sched_algo}/*.data; do
        name="$(basename $i .data)"

        cp ${tests_dir}/${sched_algo}/${name}.data \
            ${tests_dir}/binheap/${sched_algo}/${name}.data
        [[ $? -eq 0 ]] && echo "Generated binheap/${sched_algo}/${name}.data"

        cp ${tests_dir}/${sched_algo}/${name}.expect \
            ${tests_dir}/binheap/${sched_algo}/${name}.expect
        [[ $? -eq 0 ]] && echo "Generated binheap/${sched_algo}/${name}.expect"
    done
done

# For the sample compiled programs (provided the corresponding JSON file exists)
# - FIFO
# - PIFO tree
# - Round robin with 2 flows
# - Strict with 3 flows
# - Round robin union, which essentially behaves like a FIFO

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/compiler/fifo_compile.data
[[ $? -eq 0 ]] && echo "Generated compiler/fifo_compile.data"

cat ${tests_dir}/compiler/fifo_compile.data \
        | python3 ${data_gen_dir}/fifo_oracle.py $num_cmds $queue_size --keepgoing \
        > ${tests_dir}/compiler/fifo_compile.expect
[[ $? -eq 0 ]] && echo "Generated compiler/fifo_compile.expect"

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/compiler/pifo_tree_compile.data
[[ $? -eq 0 ]] && echo "Generated compiler/pifo_tree_compile.data"

cat ${tests_dir}/compiler/pifo_tree_compile.data \
        | python3 ${data_gen_dir}/pifo_tree_oracle.py $num_cmds $queue_size --keepgoing \
        > ${tests_dir}/compiler/pifo_tree_compile.expect
[[ $? -eq 0 ]] && echo "Generated compiler/pifo_tree_compile.expect"

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/compiler/rr_compile.data
[[ $? -eq 0 ]] && echo "Generated compiler/rr_compile.data"

cat ${tests_dir}/compiler/rr_compile.data \
        | python3 ${data_gen_dir}/rr_oracle.py $num_cmds $queue_size 2 --keepgoing \
        > ${tests_dir}/compiler/rr_compile.expect
[[ $? -eq 0 ]] && echo "Generated compiler/rr_compile.expect"

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/compiler/strict_compile.data
[[ $? -eq 0 ]] && echo "Generated compiler/strict_compile.data"

cat ${tests_dir}/compiler/strict_compile.data \
        | python3 ${data_gen_dir}/strict_oracle.py $num_cmds $queue_size 3 --keepgoing \
        > ${tests_dir}/compiler/strict_compile.expect
[[ $? -eq 0 ]] && echo "Generated compiler/strict_compile.expect"

python3 ${data_gen_dir}/gen_oracle_data.py $num_cmds \
        > ${tests_dir}/compiler/fifo_union_compile.data
[[ $? -eq 0 ]] && echo "Generated compiler/fifo_union_compile.data"

cat ${tests_dir}/compiler/fifo_union_compile.data \
        | python3 ${data_gen_dir}/fifo_oracle.py $num_cmds $queue_size --keepgoing \
        > ${tests_dir}/compiler/fifo_union_compile.expect
[[ $? -eq 0 ]] && echo "Generated compiler/fifo_union_compile.expect"