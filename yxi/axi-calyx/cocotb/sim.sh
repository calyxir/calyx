#!/bin/bash

# Intended to convert from calyx to synthesizable verilog, enable waveform tracing and run tests defined in Makefile
#expects an outputs/ dir one level up from here
fud e ../generated-axi-with-vec-add.futil --from calyx --to synth-verilog -o ../outputs/generated-axi-with-vec-add.v \
    && ../vcdump.py ../outputs/generated-axi-with-vec-add.v \
    && make WAVES=1 \
    && mv out.vcd generated-axi-with-vec-add.fst 
