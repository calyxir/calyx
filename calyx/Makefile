.PHONY: build test run test clean

build:
	cargo clippy

test:
	cargo clippy
	cargo test -- --nocapture

run: build
	cargo run stdlib.rkt and3way -l ./primitives/std.lib

install:
	cargo install --force --path .

clean:
	rm -f sim/*.vcd
	rm -f sim/*.v

%.v: src/*
	cargo run examples/$*.futil -l primitives/std.lib -o sim/$*.v

%.vcd: sim/%.v
	echo $*
	verilator -cc --trace sim/$*.v --exe sim/testbench.cpp
	make -j -C obj_dir -f V$*.mk V$*
	obj_dir/V$*
	rm -rf obj_dir
