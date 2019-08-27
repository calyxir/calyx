.PHONY: clean all all-json all-fuse all-futil status

JSON_URL:=https://raw.githubusercontent.com/nlohmann/json/develop/single_include/nlohmann/json.hpp
SOURCES:=$(wildcard *.json)

all: $(patsubst %.json,%.run,$(SOURCES))

status:
	@echo "`ls -l *.run | wc -l`/`ls -l *.fuse | wc -l` successfully ran"
	@echo "`cat *.run | grep success | wc -l`/`ls -l *.run | wc -l` passed"
	@echo "failed:"
	@grep "failure" *.run | cut -f 1 -d ':' | cut -f 1 -d '.' | sed -e 's/^/ - /;'
	@echo "succeeded:"
	@grep "success" *.run | cut -f 1 -d ':' | cut -f 1 -d '.' | sed -e 's/^/ - /;'

all-fuse: $(patsubst %.json,%.run-fuse,$(SOURCES))

all-futil: $(patsubst %.json,%.run-futil,$(SOURCES))

all-json: $(patsubst %.data,%.json,$(wildcard *.data))

%.cpp: %.fuse.pre %.json
	fuse run $< -o $@ -x "-Iprint_headers/"
	@if [ ! -f _headers/json.hpp ]; then wget $(JSON_URL) --directory-prefix _headers; fi

%.fuse.pre: %.fuse
	m4 $< > $@

%.futil: %.fuse.pre
	fuse -b futil $< -o $@

%.run-fuse: %.cpp
	./$(addsuffix .cpp.o,$*) $(addsuffix .json,$*) > $@

%.run-futil: %.futil
	racket $< -c $(addsuffix .json,$*) > $@

%.json:
	racket $(addsuffix .data,$*)

%.run: %.run-fuse %.run-futil
	@if ./diff.py $?; then \
		echo -e "[\e[32mBENCHMARK\e[0m] $* passed!"; \
		echo success > $(addsuffix .run,$*); \
	else \
		echo -e "[\e[31mBENCHMARK\e[0m] $* failed!"; \
		echo failure > $(addsuffix .run,$*); \
	fi

clean:
	rm -rf *.cpp *.o _headers *.out *.run-fuse *.run-futil *.run *.futil *.pre
