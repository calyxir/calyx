# Resource Estimation Backend

The resources estimation backend aims to provide a size estimation for the hardware that Calyx generates. Currently, it only supports estimation for `std_reg`, `comb_mem_*`, and `seq_mem_*` primitives, but more primitives will be added.

## Running the resource estimation backend

1. Run `cargo build` if you haven't built the compiler already.
2. Run `fud e path/to/futil.file --to resources`. This should tally up the primitives used in the program and output a CSV with the number of instantiated primitives according to their attributes.

To output the CSV to a file, you can use `-o myfile.csv`.
If you would like to see an English summary of the CSV as well as the estimated size of the hardware (counting only the supported primitives), add the verbose flag `-vv` to your `fud` command.
