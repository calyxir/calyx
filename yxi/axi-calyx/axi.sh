#!/bin/bash
# run from calyx dir
# Get the original program’s .yxi interface.
# Generate a wrapper based on that interface.
# Integrate between the wrapper and the original program.
target/debug/calyx $1 -b yxi > yxi/axi-calyx/fud-input.yxi 
target/debug/calyx $1 -p external-to-ref > yxi/axi-calyx/fud-after-pass.futil
cd yxi/axi-calyx
python3 axi-generator.py interface.yxi > fud-generated-axi.futil
cat fud-generated-axi.futil fud-after-pass.futil > fud-cat.futil

fud e fud-cat.futil --from calyx --to synth-verilog -o ../outputs/$2.v \
    && ../vcdump.py ../outputs/$2.v \
    && make WAVES=1 vfile=../outputs/$2.v\
    && mv out.vcd ../outputs/$2.fst 

rm fud-input.yxi
rm fud-generated-axi.futil
rm fud-after-pass.futil
# rm fud-cat.futil
