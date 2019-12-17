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

%.v:
	cargo run examples/$*.futil -l primitives/std.lib -o $*.v

%.vcd: %.v
	verilator -cc --trace $*.v --exe sim/testbench.cpp --top-module main
	make -j -C obj_dir -f Vmain.mk Vmain
	obj_dir/Vmain $*.vcd
	rm -rf obj_dir
