TESTS := ./test/*.btor2

.PHONY: install
install:
	RUSTFLAGS="-C target-cpu=native" cargo install --path .

.PHONY: test
test:
	turnt $(TESTS)

.PHONY: benchmarks
benchmarks:
	python3 brench-pipeless/brench.py benchmark.toml

# This is primarily used for running examples and debuging a bril program
.PHONY: example
example:
	bril2json < ../benchmarks/sqrt.bril | cargo run
