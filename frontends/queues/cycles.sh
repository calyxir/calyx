#!/usr/bin/bash

shopt -s globstar

cd "$(dirname "$0")/../.." # move to root

declare -a files=(frontends/queues/tests/**/*.py)
num_files=${#files[@]}

echo "{"

for (( i=0; i<${num_files}; i++ )); do
    file="${files[$i]}"
    name="$(basename $file .py)"
    dir="$(dirname $file)"
    
    cycles="$(python3 $file 20000 --keepgoing |\
                fud e --from calyx --to jq \
                    --through verilog \
                    --through dat \
                    -s verilog.data "$dir/$name.data" \
                    -s jq.expr ".cycles" \
                    -q)"

    echo -n "\"${file#*tests/}\" : $cycles"

    # because JSON doesn't allow trailing ','s
    if [ $i -ne $(( num_files - 1 )) ]; then 
        echo ","
    else
        echo ""
    fi
done

echo "}"
