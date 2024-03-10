#!/bin/bash
# run from calyx dir
# Get the original program’s .yxi interface.
# Generate a wrapper based on that interface.
# Integrate between the wrapper and the original program.

# get yxi
target/debug/calyx $1 -b yxi > yxi/axi-calyx/outputs/fudaxi/input.yxi 
# external to ref pass - doesn't have same behavior as sim.sh workflow
target/debug/calyx $1 -p external-to-ref > yxi/axi-calyx/outputs/fudaxi/after-pass.futil
cd yxi/axi-calyx
# 
python3 axi-generator.py outputs/fudaxi/input.yxi > outputs/fudaxi/generated-axi.futil
cd outputs/fudaxi
cat generated-axi.futil after-pass.futil > cat.futil

cd ../../cocotb 
fud e ../outputs/fudaxi/generated-axi.futil --from calyx --to synth-verilog -o ../outputs/$2.v 
../vcdump.py ../outputs/$2.v 
make WAVES=1 vfile=../outputs/$2.v 
mv out.vcd ../outputs/$2.fst 

# fud e fudaxi-cat.futil --from calyx --to synth-verilog -o ../outputs/$2.v \
#     && ../vcdump.py ../outputs/$2.v \
#     && make WAVES=1 vfile=../outputs/$2.v\
#     && mv out.vcd ../outputs/$2.fst 

# rm fudaxi/input.yxi
# rm fudaxi/generated-axi.futil
# rm fudaxi/after-pass.futil
# rm fudaxi/cat.futil
