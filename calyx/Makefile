.PHONY: build test run test

build:
	cargo build

test:
	RUST_BACKTRACE=1 cargo test -- --nocapture

run:
	RUST_BACKTRACE=1 cargo run stdlib.rkt -l ./primitives/std.lib

install:
	cargo install --path .
