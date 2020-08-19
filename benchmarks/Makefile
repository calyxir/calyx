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

small_polybench/%.cpp.o: small_polybench/%.fuse
	../bin/find-dahlia run $< -o small_polybench/$*.cpp -l error --lower
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi
	rm -f small_polybench/$*.cpp # remove extraneously generated file

small_polybench/%.dahlia-out: small_polybench/%.cpp.o small_polybench/%.fuse.data
	./$< small_polybench/$*.fuse.data | jq -S . > $@

small_polybench/%.meminit: small_polybench/%.fuse.data
	../bin/json_to_dat.py --mode json --output $@ $<

small_polybench/%.futil: small_polybench/%.fuse
	../bin/find-dahlia $< --lower -b futil -l error > $@

small_polybench/%.sv: small_polybench/%.futil
	cargo run -- $< -l .. -b verilog --verilator > $@

small_polybench/%.vcd: small_polybench/%.meminit small_polybench/%.sv
	DATA=small_polybench/$*.meminit ../bin/gen-vcd small_polybench/$*.sv > $@

small_polybench/%.futil-out: small_polybench/%.meminit small_polybench/%.vcd
	../bin/json_to_dat.py --mode dat --output $@.tmp small_polybench/$*.meminit
	jq -S . $@.tmp > $@
	rm $@.tmp

small_polybench/%.diff: small_polybench/%.dahlia-out small_polybench/%.futil-out
	diff small_polybench/$*.dahlia-out small_polybench/$*.futil-out > $@

clean:
	rm -rf small_polybench/*.cpp small_polybench/*.o small_polybench/*.futil small_polybench/*.dahlia-out small_polybench/*.futil-out small_polybench/*.sv small_polybench/*.vcd small_polybench/*.diff small_polybench/*.meminit
