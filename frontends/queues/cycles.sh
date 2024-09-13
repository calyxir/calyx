#!/usr/bin/bash

shopt -s globstar

cd "$(dirname "$0")/../.." # move to root

for file in frontends/queues/tests/**/*.py; do
    name="$(basename $file .py)"
    dir="$(dirname $file)"
    
    cycles="$(python3 $file 20000 --keepgoing |\
                fud e --from calyx --to jq \
                    --through verilog \
                    --through dat \
                    -s verilog.data "$dir/$name.data" \
                    -s jq.expr ".cycles" \
                    -q)"
    echo "${file#*tests/}: $cycles"
done
