#!/bin/bash

mkdir -p benchmarks/hls

# Run from the root directory.
cd calyx-backend/src/egg/evaluation/polybench-dahlia

for file in *$path.fuse; do
    # Extract the name without extension
    name="${file%.*}"
    cd ../polybench-calyx

    fud e -q ../polybench-dahlia/$file --to calyx > $name.futil
done
