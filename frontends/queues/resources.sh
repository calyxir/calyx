#!/usr/bin/bash

shopt -s globstar

if [ "$#" -gt 1 ]; then
    echo "usage: ./resources.sh [resource]"
    exit 1
fi

cd "$(dirname "$0")/../.." # move to root

for file in frontends/queues/tests/**/*.py; do
    name="$(basename $file .py)"
    dir="$(dirname $file)"
    
    resources="$(python3 $file 20000 --keepgoing |\
                fud e --from calyx --to resource-estimate)"

    if [ "$#" -eq 1 ]; then
        resource=$(jq ".$1" <<< "$resources")
        echo "${file#*tests/}: $resource"
    else
        newline=$'\n'
        echo "${file#*tests/}${newline}$resources${newline}"
    fi
done
