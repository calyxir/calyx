.PHONY: clean all

SOURCES:=$(wildcard *.fuse)
OBJS:=$(patsubst %.fuse,%.cpp,$(SOURCES))

all: $(OBJS)

%.data: %.cpp
	@echo Fuse:
	@./$(addsuffix .cpp.o,$(basename $@)) $(addsuffix .data,$(basename $@))
	@echo FuTIL:
	@racket $(addsuffix .rkt,$(basename $@)) $(addsuffix .data,$(basename $@))

%.cpp: %.fuse.pre
	fuse run $< -o $@

%.fuse.pre: %.fuse
	m4 $< > $@

clean:
	rm -f *.cpp *.o
