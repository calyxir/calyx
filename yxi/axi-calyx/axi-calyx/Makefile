.PHONY: test
test: test.yxi
	cargo run --manifest-path "Cargo.toml" test.yxi

test.yxi: test.futil
	cargo run --manifest-path "../../../tools/yxi/Cargo.toml" -- -l "../../../calyx-stdlib/primitives" $< > $@
