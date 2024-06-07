.PHONY: run
run:
	cd .. && cargo build
	calyx-pass -e ../target/debug/calyx -c main -b tdcc test.futil
