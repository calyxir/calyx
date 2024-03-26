# btor2i

`btor2i` is a faster interpreter for BTOR2 written in rust.

It is available as both a command-line interface and a rust library.

## Command-line interface

Using `cargo`; run `cargo install --path .` and make sure `$HOME/.cargo/bin` is 
on your path.  

Run `btor2i --help` for all of the supported flags.

## Rust interface [WIP]

`btor2i` can also be used in your rust code which may be advantageous. Add `btor2i`
 to your `Cargo.toml` with:

```toml
[dependencies.btor2i]
version      = "0.1.0"
path         = "../btor2i"
```

Check out `cargo doc --open` for exposed functions. 

## Contributing

Issues and PRs are welcome. For pull requests, make sure to run the [Turnt](https://github.com/cucapra/turnt)
test harness with `make test` and `make benchmark`. The `make test` output is in 
[TAP](https://testanything.org/) format and can be prettified with TAP consumers,
like [Faucet](https://github.com/ljharb/faucet). There is also `.github/workflows/rust.yaml` 
which will format your code and check  that it is conforming with clippy.
