.PHONY: clean

clean:
	rm -f *.vcd *.v *.futil *.dot *.png

%.futil: %.fuse
	fuse -b futil $< > $@

%.v: examples/%.futil
	echo '/* verilator lint_off PINMISSING */' > $@
	echo '`include "sim/lib/std.v"' >> $@
	echo '\n' >> $@
	cargo run -- $< -l primitives/std.lib -b verilog >> $@

%.vcd: %.v
	verilator -cc --trace $< --exe sim/testbench.cpp --top-module main
	make -j -C obj_dir -f Vmain.mk Vmain
	obj_dir/Vmain $@
	rm -rf obj_dir
