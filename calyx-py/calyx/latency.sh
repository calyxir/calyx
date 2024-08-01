#!/bin/bash

# The unannotated code is in sdn.futil, and the annotated code is in sdn_static.futil.
# The data file is sdn.data.
code=../test/correctness/queues/fifo.futil #might have to generate .futil file
#annotated_code=benchmarks/sdn/sdn_static.futil
data=../test/correctness/queues/fifo.data
# By default, the code is promoted to Piezo. To disable promotion, we pass the -d static-promotion flag.

# Unannotated code, not further promoted
# Cycle counts
fud e -q $code --to dat --from calyx --through verilog -s verilog.data $data -s calyx.flags ' -d static-promotion ' | jq '{ "latency": .cycles }'  > results/fifo_latency.json
