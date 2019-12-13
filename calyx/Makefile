.PHONY: build test run test

build:
	cargo clippy

test:
	cargo clippy
	cargo test -- --nocapture

run: build
	cargo run stdlib.rkt and3way -l ./primitives/std.lib

install:
	cargo install --force --path .
