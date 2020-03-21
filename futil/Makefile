.PHONY: test build install watch

test:
	racket test/unit-tests.rkt

build:
	raco make main.rkt
	@echo "done"

watch:
	while true; do find . | entr -cd make build; test $? -gt 128 && break; done

install:
	raco pkg install

uninstall:
	raco pkg remove .
