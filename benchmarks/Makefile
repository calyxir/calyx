.PHONY: clean all

JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp
SOURCES:=$(wildcard *.data)

all: $(patsubst %.data,%.run,$(SOURCES))

%.cpp: %.fuse.pre
	fuse run $< -o $@ -x "-Iprint_headers/"
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi

%.fuse.pre: %.fuse
	m4 $< > $@

%.futil: %.fuse.pre
	fuse -b futil $< -o $@

%.run-fuse: %.cpp
	./$(addsuffix .cpp.o,$*) $(addsuffix .data,$*) >> $@

%.run-futil: %.futil
	racket $< -c $(addsuffix .data,$*) >> $@

%.run: %.run-fuse %.run-futil
	@if diff -y $?; then \
		echo -e "[\e[32mTEST\e[0m] $* passed!"; \
	else \
		echo -e "[\e[31mTEST\e[0m] $* failed!"; \
	fi

clean:
	rm -rf *.cpp *.o _headers *.out *.run-fuse *.run-futil *.run-futil-hand *.futil *.pre
