.PHONY: build test run test

build:
	cargo clippy
	# cargo build

test:
	cargo clippy
	cargo test -- --nocapture

run:
	cargo clippy
	cargo run stdlib.rkt and3way -l ./primitives/std.lib

install:
	cargo install --path .
