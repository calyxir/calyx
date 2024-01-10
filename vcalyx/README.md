# VCalyx

## Parsing a single file

Install flit:

    python3 -m pip install flit

In the `fud` directory, run:

    flit install --symlink

Finally, run the `fud` command:

    fud e <path/to/futil> --to vcalyx

To obtain the S-expression form of a Calyx program, run:

    fud e <path/to/futil> --to vcalyx-sexp 

## Running the test suite

Install runt:

    cargo install runt

In the root directory of the repo, run:

    runt -d vcalyx
