.PHONY: build test run test

build:
	cargo build

test:
	cargo test -- --nocapture

run:
	cargo run stdlib.rkt and3way -l ./primitives/std.lib

install:
	cargo install --path .
