.PHONY: clean

JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp

small_polybench/%.fuse: templates/small_polybench/%.fuse.header
	cat $< templates/common_polybench/$*.fuse.pre | m4 > $@

large_polybench/%.fuse: templates/large_polybench/%.fuse.header
	cat $< templates/common_polybench/$*.fuse.pre | m4 > $@

small_polybench/%.fuse.data: templates/small_polybench/%.template
	../bin/gen_data.py $< > $@

large_polybench/%.fuse.data: templates/large_polybench/%.template
	../bin/gen_data.py $< > $@

# rule to make cpp from fuse
%.cpp: %.fuse
	../bin/find-dahlia run $< -o $@ -l error --lower
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi
	rm -f $@.o # remove extraneously generated file

%.cpp.o: %.fuse
	../bin/find-dahlia run $< -o $*.cpp -l error --lower
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi
	rm -f $*.cpp # remove extraneously generated file

%.dahlia-out: %.cpp.o %.data
	./$< $*.data | jq -S . > $@

small_polybench/%.meminit: small_polybench/%.data
	../bin/json_to_dat.py --mode json --output $@ $<

%.futil: %.fuse
	../bin/find-dahlia $< --lower -b futil -l error > $@

%.sv: %.futil
	cargo run -- $< -l .. -b verilog --verilator > $@

%.vcd: %.meminit %.sv
	DATA=$*.meminit ../bin/gen-vcd $*.sv > $@

%.futil-out: %.meminit %.vcd
	../bin/json_to_dat.py --mode dat --output $@.tmp $*.meminit
	jq -S . $@.tmp > $@
	rm $@.tmp

%.diff: %.dahlia-out %.futil-out
	diff $*.dahlia-out $*.futil-out > $@

clean:
	rm -rf *.cpp *.o *.run *.futil *.dahlia-out *.futil-out *.json *.sv *.vcd *.diff *.meminit *.data
