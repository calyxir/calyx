.PHONY: clean all

JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp
SOURCES:=$(wildcard *.data)

all: $(patsubst %.data,%.out,$(SOURCES))

%.out: %.cpp
	@echo Fuse: >> $@
	@./$(addsuffix .cpp.o,$(basename $@)) $(addsuffix .data,$(basename $@)) >> $@
	@echo Futil: >> $@
	@racket $(addsuffix .rkt,$(basename $@)) $(addsuffix .data,$(basename $@)) >> $@
	@cat $@
	rm $@

%.cpp: %.fuse.pre
	fuse run $< -o $@ -x "-Iprint_headers/"
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi

%.fuse.pre: %.fuse
	m4 $< > $@

clean:
	rm -rf *.cpp *.o _headers *.out
