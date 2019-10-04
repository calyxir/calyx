.PHONY: build test

build:
	cargo build

test:
	RUST_BACKTRACE=1 cargo test -- --nocapture

run:
	RUST_BACKTRACE=1 cargo run stdlib.rkt