#!/bin/bash

# Intended to convert from calyx to synthesizable verilog, enable waveform tracing and run tests defined in Makefile
#expects an outputs/ dir one level up from here
fud e ../axi-combined-calyx.futil --from calyx --to synth-verilog -o ../outputs/axi-combined.v \
    && ../vcdump.py ../outputs/axi-combined.v \
    && make WAVES=1 \
    && mv out.vcd axi-combined.fst 
