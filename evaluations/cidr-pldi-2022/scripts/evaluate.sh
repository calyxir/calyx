#!/bin/bash

# Gathers simulation times for Calyx interprer, Verilog, and Icarus-Verilog.
# Example:
#   PROGRAM="examples/futil/dot-product.futil"
#      DATA="examples/dahlia/dot-product.fuse.data"
#      FILE="dot-product.csv"
# INTERVALS=10
  PROGRAM=$1
     DATA=$2
     FILE=$3
INTERVALS=$4

# Gather Calyx interpreter simulation times.
for (( i = 0; i < $INTERVALS; ++i ))
do
    fud e $PROGRAM --to interpreter-out -s verilog.data $DATA \
    -pr interpreter.interpret -csv \
    >> $FILE
done

# Gather Icarus-Verilog simulation times.
for (( i = 0; i < $INTERVALS; ++i ))
do
    fud e $PROGRAM --to dat -s verilog.data $DATA --through icarus-verilog \
    -pr icarus-verilog.simulate icarus-verilog.compile_with_iverilog -csv \
    >> $FILE
done

# Gather Verilog simulation times.
for (( i = 0; i < $INTERVALS; ++i ))
do
    fud e $PROGRAM --to dat -s verilog.data $DATA \
    -pr verilog.simulate verilog.compile_with_verilator -csv \
    >> $FILE
done

