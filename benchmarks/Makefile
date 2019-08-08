.PHONY: clean all

SOURCES:=$(wildcard *.fuse)
OBJS:=$(patsubst %.fuse,%.cpp,$(SOURCES))
JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp

all: $(OBJS)

%.data: %.cpp _headers/json.hpp
	@echo Fuse:
	@./$(addsuffix .cpp.o,$(basename $@)) $(addsuffix .data,$(basename $@))
	@echo Futil:
	@racket $(addsuffix .rkt,$(basename $@)) $(addsuffix .data,$(basename $@))

_headers/json.hpp:
	wget $(JSON_URL) --directory-prefix _headers

%.cpp: %.fuse.pre
	fuse run $< -o $@ -x "-Iprint_headers/"

%.fuse.pre: %.fuse
	m4 $< > $@

clean:
	rm -rf *.cpp *.o _headers
