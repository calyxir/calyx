#!/bin/bash
# run from calyx dir
# Get the original program’s .yxi interface.
# Generate a wrapper based on that interface.
# Integrate between the wrapper and the original program.

#run from calyx dir with 
#bash yxi/axi-calyx/axi.sh yxi/axi-calyx/fixed-vec-add-w-imports.futil test
#takes fixed-vec-add-w-imports.futil (a "normal" calyx file) 
#and outputs compiled verilog to test.v and waveforms to test.fst

# get yxi
target/debug/calyx $1 -b yxi > yxi/axi-calyx/outputs/fudaxi/1input.yxi 
# external to ref pass - doesn't have same behavior as sim.sh workflow
target/debug/calyx $1 -p external-to-ref > yxi/axi-calyx/outputs/fudaxi/2after-pass.futil
cd yxi/axi-calyx
# 
python3 axi-generator.py outputs/fudaxi/1input.yxi > outputs/fudaxi/3generated-axi.futil
cd outputs/fudaxi
cat 3generated-axi.futil 2after-pass.futil > 4cat.futil
python3 ../../remove-imports.py 4cat.futil 5noimports.futil 
cd ../../cocotb 
touch ../outputs/$2.v 
fud e ../outputs/fudaxi/5noimports.futil --from calyx --to synth-verilog -o ../outputs/$2.v 
../vcdump.py ../outputs/$2.v 
make WAVES=1 vfile=../outputs/$2.v 
mv out.vcd ../outputs/$2.fst 
cd ../outputs/fudaxi
rm 1input.yxi
rm 2after-pass.futil
rm 3generated-axi.futil
rm 4cat.futil
rm 5noimports.futil

