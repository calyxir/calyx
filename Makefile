.PHONY: clean install

target/debug/futil:
	cargo build

install:
	cargo build
	ln -sr ./target/debug/futil ~/.local/bin/futil
