.PHONY: clean all all-json all-fuse all-futil status

JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp
SOURCES:=$(wildcard *.fuse)

all: $(patsubst %.fuse,%.diff,$(SOURCES))

status:
	@echo "`ls -l *.run | wc -l`/`ls -l *.fuse | wc -l` successfully ran"
	@echo "`cat *.run | grep success | wc -l`/`ls -l *.run | wc -l` passed"
	@echo "failed:"
	@grep "failure" *.run | cut -f 1 -d ':' | cut -f 1 -d '.' | sed -e 's/^/ - /;'
	@echo "succeeded:"
	@grep "success" *.run | cut -f 1 -d ':' | cut -f 1 -d '.' | sed -e 's/^/ - /;'

%.cpp: %.fuse
	../bin/find-dahlia run $< -o $@
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi
	rm -f $@.o # remove extraneously generated file

%.cpp.o: %.fuse
	../bin/find-dahlia run $< -o $*.cpp
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi
	rm -f $*.cpp # remove extraneously generated file

%.dahlia-out: %.cpp.o
	./$< $*.data | jq -S . > $@

%.futil: %.fuse
	../bin/find-dahlia $< --lower -b futil | sed 's/<"static"=1>//g' > $@

%.sv: %.futil
	cargo run -- $< -l .. -b verilog --verilator > $@

%.meminit: data/%.data
	../bin/json_to_dat.py --mode json --output $@ $<

%.vcd: %.meminit %.sv
	DATA=$*.meminit ../bin/gen-vcd $*.sv > $@

%.futil-out: %.meminit %.vcd
	../bin/json_to_dat.py --mode dat --output $@.tmp $*.meminit
	jq -S . $@.tmp > $@
	rm $@.tmp

%.diff: %.dahlia-out %.futil-out
	diff $*.dahlia-out $*.futil-out > $@

clean:
	rm -rf *.cpp *.o *.run *.futil *.dahlia-out *.futil-out *.json *.sv *.vcd _headers *.diff *.meminit
