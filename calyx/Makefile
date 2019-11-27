.PHONY: build test run test

build:
	cargo build

test:
	cargo test -- --nocapture

run:
	cargo run stdlib.rkt -l ./primitives/std.lib

install:
	cargo install --path .
