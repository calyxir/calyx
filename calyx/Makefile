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
	rm */*.vcd

%.v:
	cargo run $*.futil -l primitives/std.lib -o $*.v

%.vcd: %.v
	echo $*
	verilator -cc --trace $*.v --exe tb.cpp
	make -j -C obj_dir -f V$*.mk V$*
	obj_dir/V$*
	rm -rf obj_dir
