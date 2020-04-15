.PHONY: clean

target/debug/futil:
	cargo build

clean:
	rm -f *.vcd *.v *.futil *.dot *.png
	rm -f tests/verilog/*.{vcd,v,res,json}

%.futil: %.fuse
	dahlia -b futil $< > $@

%.v: %.futil
	./target/debug/futil $< -l primitives/std.lib -b verilog > $@

%.vcd: %.v
	mkdir -p $*_objs
	cp sim/testbench.cpp $*_objs/testbench.cpp
	verilator -cc --trace $< --exe testbench.cpp --top-module main --Mdir $*_objs
	make -j -C $*_objs -f Vmain.mk Vmain
	$*_objs/Vmain $@
	rm -rf $*_objs

%.json: %.vcd
	vcdump $< > $*.json

%.res: %.json
	cat $< | jq -f $*.jq > $*.res
