#!/usr/bin/bash

shopt -s globstar

if [ "$#" -gt 1 ]; then
    echo "usage: ./resources.sh [resource]"
    exit 1
fi

cd "$(dirname "$0")/../.." # move to root

declare -a files=(frontends/queues/tests/**/*.py)
num_files=${#files[@]}

echo "{"

for (( i=0; i<${num_files}; i++ )); do
    file="${files[$i]}"
    name="$(basename $file .py)"
    dir="$(dirname $file)"
    
    resources="$(python3 $file 20000 --keepgoing |\
                fud e --from calyx --to resource-estimate)"

    if [ "$#" -eq 1 ]; then
        resource=$(jq ".$1" <<< "$resources")
        echo -n "\"${file#*tests/}\" : $resource"
    else
        echo "\"${file#*tests/}\" :" 
        echo -n "$resources"
    fi

    # because JSON doesn't allow trailing ','s
    if [ $i -ne $(( num_files - 1 )) ]; then
        echo ","
    else
        echo ""
    fi
done

echo "}"
