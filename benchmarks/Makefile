.PHONY: clean all

JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp
SOURCES:=$(wildcard *.data)

all: $(patsubst %.data,%.run,$(SOURCES))

%.run-fuse: %.cpp
	./$(addsuffix .cpp.o,$*) $(addsuffix .data,$*) >> $@

%.run-futil:
	racket $(addsuffix .rkt,$*) $(addsuffix .data,$*) >> $@

%.run: %.run-fuse %.run-futil
	@if diff -y $?; then \
		echo "[TEST] $* passed!"; \
	else \
		echo "[TEST] $* failed!"; \
	fi

%.cpp: %.fuse.pre
	fuse run $< -o $@ -x "-Iprint_headers/"
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi

%.fuse.pre: %.fuse
	m4 $< > $@

clean:
	rm -rf *.cpp *.o _headers *.out *.run-fuse *.run-futil
