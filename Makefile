.PHONY: test build install watch

test:
	racket test/unit-tests.rkt

build:
	cd futil; raco make main.rkt
	@echo "done"

watch:
	find futil | entr -cd make build; make watch

install:
	cd futil; raco pkg install

uninstall:
	raco pkg remove futil
