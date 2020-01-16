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
	rm -f *.vcd sim/*.vcd
	rm -f *.v sim/*.v
	rm -f *.dot

%.v: src/*
	cargo run examples/$*.futil -l primitives/std.lib -o sim/$*.v

%.vcd: sim/%.v
	echo $*
	verilator -cc --trace sim/$*.v --exe sim/testbench.cpp --top-module main
	make -j -C obj_dir -f Vmain.mk Vmain
	obj_dir/Vmain
	rm -rf obj_dir
