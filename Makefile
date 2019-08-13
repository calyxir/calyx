.PHONY: test build install

test:
	racket test/unit-tests.rkt

build:
	cd futil; raco make main.rkt

install:
	cd futil; raco pkg install

uninstall:
	cd futil; raco pkg remove
