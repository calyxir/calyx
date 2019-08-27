.PHONY: test build install watch

test:
	racket test/unit-tests.rkt

build:
	cd futil; raco make main.rkt
	@echo "done"

watch:
	while true; do find futil | entr -cd make build; test $? -gt 128 && break; done

install:
	cd futil; raco pkg install

uninstall:
	raco pkg remove futil
