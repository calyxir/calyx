#!/usr/bin/bash

# For SDN, we use piezo mode when making the data file and
# use pifotree_oracle to generate the expected output
python3 queue_data_gen.py --no-err > ../test/correctness/queues/sdn.data
cat ../test/correctness/queues/sdn.data | python3 pifo_tree_oracle.py > ../test/correctness/queues/sdn.expect

# For the others, we drop piezo mode for data gen, and we use the appropriate
# oracle, which is one of the following:
# - fifo_oracle.py
# - pifo_oracle.py
# - pifo_tree_oracle.py

for queue_kind in fifo pifo pifo_tree; do
    python3 queue_data_gen.py --no-err > ../test/correctness/queues/$queue_kind.data
    cat ../test/correctness/queues/$queue_kind.data | python3 ${queue_kind}_oracle.py > ../test/correctness/queues/$queue_kind.expect
done
