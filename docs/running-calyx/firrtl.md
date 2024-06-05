# FIRRTL Backend

Calyx programs can be translated into the [FIRRTL](https://bar.eecs.berkeley.edu/projects/firrtl.html) intermediate language. <!-- TODO: Link OSDA paper when it goes on arxiv -->

## Basic Use

To translate an example program to FIRRTL, try:

    $ cargo run examples/tutorial/language-tutorial-iterate.futil -p external-to-ref > language-tutorial-iterate-ref.futil
    $ cargo run language-tutorial-iterate-ref.futil -b firrtl

## Running with `fud2`



<!--
The FIRRTL backend is best run through [fud2][], which all of our examples will use.
-->

### Setup

To run FIRRTL-translated programs, we need to set up [Firtool](https://github.com/llvm/circt) for use by fud2. We recommend using [Firtool version 1.75.0](https://github.com/llvm/circt/releases/tag/firtool-1.75.0).

First, download and extract the Firtool binary.

Then, edit `fud2`'s configuration file:

    $ fud2 edit-config

Add these lines:

```toml
[firrtl]
firtool = "<path to extracted firtool directory>/bin/firtool"
```

[fud2]: ./fud2.md

Lastly, build Calyx with the YXI feature by running the following from the Calyx root directory:

    $ cargo build --features yxi

## 