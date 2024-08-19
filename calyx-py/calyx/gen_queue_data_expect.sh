#!/usr/bin/bash

num_cmds=20000
queue_size=16

# For SDN, we use piezo mode when making the data file and
# use pifotree_oracle to generate the expected output
python3 queue_data_gen.py $num_cmds --no-err $queue_size > ../test/correctness/queues/sdn.data
cat ../test/correctness/queues/sdn.data | python3 pifo_tree_oracle.py $num_cmds $queue_size --keepgoing > ../test/correctness/queues/sdn.expect

# For the others, we drop piezo mode for data gen, and we use the appropriate
# oracle, which is one of the following:
# - fifo_oracle.py
# - pifo_oracle.py
# - pifo_tree_oracle.py

for queue_kind in fifo pifo pifo_tree complex_tree; do
    python3 queue_data_gen.py $num_cmds > ../test/correctness/queues/$queue_kind.data
    [[ "$queue_kind" != "pifo_tree" ]] && cp ../test/correctness/queues/$queue_kind.data ../test/correctness/queues/binheap/$queue_kind.data
    cat ../test/correctness/queues/$queue_kind.data | python3 ${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing > ../test/correctness/queues/$queue_kind.expect
    [[ "$queue_kind" != "pifo_tree" ]] && cp ../test/correctness/queues/$queue_kind.expect ../test/correctness/queues/binheap/$queue_kind.expect
done

# Here, we test the queues for non-work-conserving algorithms,
# which are the following:
# - pieo_oracle.py
# - pcq_oracle.py

for queue_kind in pieo pcq nwc_simple; do
    python3 queue_data_gen.py $num_cmds --nwc-en > ../test/correctness/queues/$queue_kind.data
    cat ../test/correctness/queues/$queue_kind.data | python3 ${queue_kind}_oracle.py $num_cmds $queue_size --keepgoing > ../test/correctness/queues/$queue_kind.expect
done  

# For the Binary Heap, we drop piezo mode and enable ranks for data gen and
# use binheap_oracle to generate the expected output
python3 queue_data_gen.py $num_cmds --use-rank > ../test/correctness/queues/binheap/stable_binheap.data
cat ../test/correctness/queues/binheap/stable_binheap.data | python3 binheap_oracle.py $num_cmds $queue_size --keepgoing > ../test/correctness/queues/binheap/stable_binheap.expect

# For the Round Robin queues, we drop piezo mode as well and use rrqueue_oracle to
# generate the expected output for queues with 2..7 flows. This generates 6 data expect file pairs.

for n in {2..7}; do
    python3 queue_data_gen.py $num_cmds > ../test/correctness/queues/strict_and_rr_queues/rr_queues/rr_queue_${n}flows.data
    cat ../test/correctness/queues/strict_and_rr_queues/rr_queues/rr_queue_${n}flows.data | python3 rr_queue_oracle.py $num_cmds $queue_size $n --keepgoing > ../test/correctness/queues/strict_and_rr_queues/rr_queues/rr_queue_${n}flows.expect
done

# For Strict queues, we use strict_queue_oracle.py to generate the expected output
# for queues with 2..6 flows, each with a different strict ordering. This generates 5
# expect file pairs.

for n in {2..6}; do
    python3 queue_data_gen.py $num_cmds > ../test/correctness/queues/strict_and_rr_queues/strict_queues/strict_${n}flows.data
    cat ../test/correctness/queues/strict_and_rr_queues/strict_queues/strict_${n}flows.data | python3 strict_queue_oracle.py $num_cmds $queue_size $n --keepgoing > ../test/correctness/queues/strict_and_rr_queues/strict_queues/strict_${n}flows.expect
done