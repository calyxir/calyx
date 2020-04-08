.PHONY: clean

clean:
	rm -f *.vcd *.v *.futil *.dot *.png
	rm -f tests/verilog/*.{vcd,v,res,json}

%.futil: %.fuse
	dahlia -b futil $< > $@

%.v: %.futil
	cargo run -- $< -l primitives/std.lib -b verilog > $@

%.vcd: %.v
	verilator -cc --trace $< --exe sim/testbench.cpp --top-module main
	make -j -C obj_dir -f Vmain.mk Vmain
	obj_dir/Vmain $@
	rm -rf obj_dir

%.json: %.vcd
	vcdump $< > $*.json

%.res: %.json
	cat $< | jq -f $*.jq > $*.res
