#!/usr/bin/bash

num_cmds=20000
queue_size=16

test_dir="$(dirname "$0")/../tests"
data_gen_dir="$(dirname "$0")/../data_gen"


# For SDN, we use piezo mode when making the data file and
# use pifotree_oracle to generate the expected output

python3 $data_gen_dir/queue_data_gen.py $num_cmds --no-err $queue_size > $test_dir/sdn_test.data
cat $test_dir/sdn_test.data | python3 $data_gen_dir/pifo_tree_oracle.py $num_cmds $queue_size --keepgoing > $test_dir/sdn_test.expect

# For the others, we drop piezo mode for data gen, and we use the appropriate
# oracle, which is one of the following:
# - fifo_oracle.py
# - pifo_oracle.py
# - pifo_tree_oracle.py

for queue_kind in fifo pifo pifo_tree complex_tree; do
    python3 $data_gen_dir/queue_data_gen.py $num_cmds > $test_dir/$queue_kind_test.data
    cat $test_dir/$queue_kind_test.data | python3 $data_gen_dir/${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing > $test_dir/$queue_kind_test.expect
done

# Copying/Moving into binheap/

cp $test_dir/fifo_test.data $test_dir/binheap/
cp $test_dir/fifo_test.expect $test_dir/binheap/
mv $test_dir/pifo_test.data $test_dir/binheap/
mv $test_dir/pifo_test.expect $test_dir/binheap/


# Here, we test the queues for non-work-conserving algorithms,
# which are the following:
# - pieo_oracle.py
# - pcq_oracle.py

for queue_kind in pieo nwc_simple; do
    python3 $data_gen_dir/queue_data_gen.py $num_cmds --nwc-en > $test_dir/$queue_kind.data
    cat $test_dir/$queue_kind.data | python3 $data_gen_dir/${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing > $test_dir/$queue_kind.expect
done  

# For the Binary Heap, we drop piezo mode and enable ranks for data gen and
# use binheap_oracle to generate the expected output

python3 $data_gen_dir/queue_data_gen.py $num_cmds --use-rank > $test_dir/binheap/stable_binheap_test.data
cat $test_dir/binheap/stable_binheap_test.data | python3 $data_gen_dir/binheap_oracle.py $num_cmds $queue_size --keepgoing > $test_dir/binheap/stable_binheap_test.expect

# For the Round Robin queues, we drop piezo mode as well and use rrqueue_oracle to
# generate the expected output for queues with 2..7 flows. This generates 6 data expect file pairs.

for n in {2..7}; do
    python3 $data_gen_dir/queue_data_gen.py $num_cmds > $test_dir/round_robin/rr_${n}flow_test.data
    cat $test_dir/round_robin/rr_${n}flow_test.data | python3 $data_gen_dir/rr_queue_oracle.py $num_cmds $queue_size $n --keepgoing > $test_dir/round_robin/rr_${n}flow_test.expect
done

# For Strict queues, we use strict_queue_oracle.py to generate the expected output
# for queues with 2..6 flows, each with a different strict ordering. This generates 5
# expect file pairs.

for n in {2..6}; do
    python3 $data_gen_dir/queue_data_gen.py $num_cmds > $test_dir/strict/strict_${n}flow_test.data
    cat $test_dir/strict/strict_${n}flow_test.data | python3 $data_gen_dir/strict_queue_oracle.py $num_cmds $queue_size $n --keepgoing > $test_dir/strict/strict_${n}flow_test.expect
done
