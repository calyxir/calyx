#!/usr/bin/bash

num_cmds=20000
queue_size=16

tests_dir="$(dirname "$0")/../tests"
data_gen_dir="$(dirname "$0")"


# For SDN, we use piezo mode when making the data file and
# use pifotree_oracle to generate the expected output

python3 $data_gen_dir/gen_oracle_data.py $num_cmds --no-err $queue_size > $tests_dir/sdn_test.data
[[ $? -eq 0 ]] && echo "Generated sdn_test.data"
cat $tests_dir/sdn_test.data | python3 $data_gen_dir/pifo_tree_oracle.py $num_cmds $queue_size --keepgoing > $tests_dir/sdn_test.expect
[[ $? -eq 0 ]] && echo "Generated sdn_test.expect"


# For the others, we drop piezo mode for data gen, and we use the appropriate
# oracle, which is one of the following:
# - fifo_oracle.py
# - pifo_oracle.py
# - pifo_tree_oracle.py

for queue_kind in fifo pifo pifo_tree complex_tree; do
    python3 $data_gen_dir/gen_oracle_data.py $num_cmds > $tests_dir/${queue_kind}_test.data
    [[ "$queue_kind" != "pifo" && $? -eq 0 ]] && echo "Generated ${queue_kind}_test.data"
    cat $tests_dir/${queue_kind}_test.data | python3 $data_gen_dir/${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing > $tests_dir/${queue_kind}_test.expect
    [[ "$queue_kind" != "pifo" && $? -eq 0 ]] && echo "Generated ${queue_kind}_test.expect"
done


# Here, we test the queues for non-work-conserving algorithms,
# which are the following:
# - pieo_oracle.py
# - pcq_oracle.py

for queue_kind in pieo nwc_simple; do
    python3 $data_gen_dir/gen_oracle_data.py $num_cmds --nwc-en > $tests_dir/${queue_kind}_test.data
    [[ $? -eq 0 ]] && echo "Generated ${queue_kind}_test.data"
    cat $tests_dir/${queue_kind}_test.data | python3 $data_gen_dir/${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing > $tests_dir/${queue_kind}_test.expect
    [[ $? -eq 0 ]] && echo "Generated ${queue_kind}_test.expect"
done  


# For the Binary Heap, we drop piezo mode and enable ranks for data gen and
# use binheap_oracle to generate the expected output

python3 $data_gen_dir/gen_oracle_data.py $num_cmds --use-rank > $tests_dir/binheap/stable_binheap_test.data
[[ $? -eq 0 ]] && echo "Generated binheap/stable_binheap_test.data"
cat $tests_dir/binheap/stable_binheap_test.data | python3 $data_gen_dir/binheap_oracle.py $num_cmds $queue_size --keepgoing > $tests_dir/binheap/stable_binheap_test.expect
[[ $? -eq 0 ]] && echo "Generated binheap/stable_binheap_test.expect"


# For the Round Robin queues, we drop piezo mode as well and use rrqueue_oracle to
# generate the expected output for queues with 2..7 flows. This generates 6 data expect file pairs.

for n in {2..7}; do
    python3 $data_gen_dir/gen_oracle_data.py $num_cmds > $tests_dir/round_robin/rr_${n}flow_test.data
    [[ $? -eq 0 ]] && echo "Generated round_robin/rr_${n}flow_test.data"
    cat $tests_dir/round_robin/rr_${n}flow_test.data | python3 $data_gen_dir/rr_queue_oracle.py $num_cmds $queue_size $n --keepgoing > $tests_dir/round_robin/rr_${n}flow_test.expect
    [[ $? -eq 0 ]] && echo "Generated round_robin/rr_${n}flow_test.expect"
done


# For Strict queues, we use strict_queue_oracle.py to generate the expected output
# for queues with 2..6 flows, each with a different strict ordering. This generates 5
# expect file pairs.

for n in {2..7}; do
    python3 $data_gen_dir/gen_oracle_data.py $num_cmds > $tests_dir/strict/strict_${n}flow_test.data
    [[ $? -eq 0 ]] && echo "Generated strict/strict_${n}flow_test.data"
    cat $tests_dir/strict/strict_${n}flow_test.data | python3 $data_gen_dir/strict_queue_oracle.py $num_cmds $queue_size $n --keepgoing > $tests_dir/strict/strict_${n}flow_test.expect
    [[ $? -eq 0 ]] && echo "Generated strict/strict_${n}flow_test.expect"
done


# Copying/Moving into binheap/

cp $tests_dir/fifo_test.data $tests_dir/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/fifo_test.data"
cp $tests_dir/fifo_test.expect $tests_dir/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/fifo_test.expect"

mv $tests_dir/pifo_test.data $tests_dir/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/pifo_test.data"
mv $tests_dir/pifo_test.expect $tests_dir/binheap/
[[ $? -eq 0 ]] && echo "Generated binheap/pifo_test.expect"

for i in $tests_dir/round_robin/*.data; do
    file="$(basename $i)"
    cp $i $tests_dir/binheap/round_robin/$file
    [[ $? -eq 0 ]] && echo "Generated binheap/round_robin/$file"
done

for i in $tests_dir/round_robin/*.expect; do
    file="$(basename $i)"
    cp $i $tests_dir/binheap/round_robin/$file
    [[ $? -eq 0 ]] && echo "Generated binheap/round_robin/$file"
done

for i in $tests_dir/strict/*.data; do
    file="$(basename $i)"
    cp $i $tests_dir/binheap/strict/$file
    [[ $? -eq 0 ]] && echo "Generated binheap/strict/$file"
done

for i in $tests_dir/strict/*.expect; do
    file="$(basename $i)"
    cp $i $tests_dir/binheap/strict/$file
    [[ $? -eq 0 ]] && echo "Generated binheap/strict/$file"
done
