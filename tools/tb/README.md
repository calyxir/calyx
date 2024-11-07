# tb: The Calyx Testbench Tool

## Contents

1. Setup
2. Usage
3. Writing a Plugin

## Setup

Run `make plugins` to build the builtin plugins (cocotb, verilator, etc.).

## Usage

There are two ways to use `tb`:

### Directly

For example, if you make sure to follow the instructions under [`examples/cocotb/doc_examples_quickstart/`](examples/cocotb/doc_examples_quickstart/),

```
make
./tb examples/cocotb/doc_examples_quickstart/my_design.sv -t examples/cocotb/doc_examples_quickstart/test_my_design.py --using cocotb
```

should run `cocotb` on the input file and harness.

### Via `fud2`

> THIS SECTION IS INVALIDATED SINCE I HAVE REMOVED FUD2 SUPPORT FOR THE TIME
> BEING.

You can follow the above steps but invoke the following command instead.

```
fud2 my_design.sv -s tb.test=test_my_design.py -s tb.using=cocotb --to tb
```

### Makefile

I've provided a [Makefile](Makefile) in this directory for local testing. Use `make` to build the `tb` executable locally.

## Testing Calyx Code

Your input file should be the calyx file you test and each `-t` test should be a
single-file Rust program using calyx-ffi to declare one or more components in
the file and then to write a single `#[calyx_ffi_tests]` module in which you
write your test functions.

## Writing a Plugin

> WE NO LONGER ALLOW DYNAMICALLY LOADING PLUGINS.

First, setup a simple rust library as you would any other, but **ensure that `lib.crate-type` is `cdylib`**.
Here, we're writing the plugin in `lib.rs`.
Remember to update the `path` in the `dependencies.tb` dependency!

```toml
[package]
name = "my-tb-plugin"
edition = "2021" # or `edition.workspace = true`

[lib]
path = "lib.rs"
crate-type = ["cdylib"]

[dependencies]
tb = { path = "path/to/tb/crate", version = "0.0.0" }
```

In the crate, you can write any auxillary code.
However, you'll need to define at least two things:

1. A type implementing `tb::plugin::Plugin`.
2. A `declare_plugin!` declaration to expose the plugin and its constructor to the outside world.

It may be helpful to look at the existing plugins for reference.
