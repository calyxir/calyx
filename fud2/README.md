# `fud`: The Calyx Build Tool

Reimplementation of the [fud][] compiler driver for Calyx.

### Installation

To install from source, run the following from `calyx/fud2`:
```
cargo install --path .
```

This will install the binary `fud2` to the default `cargo` binary location.

### Configuration

The minimal required configuration requires setting the `calyx.base` key so that `fud` knows where the Calyx compiler is. Open the configuration file by running:
```
fud edit-config
```

Add the path to the location of the Calyx compiler:
```toml
[calyx]
base = "<path to calyx repo>"
```

[fud]: https://docs.calyxir.org/fud/index.html

### CLI

- You can pass/override config variables by passing one or more options of the form `--set variable=value`.

### Writing a new state

Given a `bld: &mut DriverBuilder`, call `bld.state`, and define appropriate rules via `bld.rule`. 
Each rule may require one or more setups; a setup can be obtained through `bld.setup` and may in addition define variables (including those mapped to config file keys) or rules.
