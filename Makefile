.PHONY: clean

target/debug/futil:
	cargo build

clean:
	rm -f *.vcd *.v *.futil *.dot *.png
	rm -f tests/verilog/*.{vcd,v,res,json}

%.futil: %.fuse
	dahlia -b futil $< > $@

%.v: %.futil
	./target/debug/futil $< -b verilog > $@

%.vcd: %.v
	./bin/gen-vcd $<

%.json: %.vcd
	vcdump $< > $*.json

%.res: %.json
	cat $< | jq -f $*.jq > $*.res
