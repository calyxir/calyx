# Btor2Tools

[![Build Status](https://github.com/ChristianMoesl/btor2tools.rs/workflows/Test/badge.svg)](https://github.com/ChristianMoesl/btor2tools.rs/actions)
[![Crate](https://img.shields.io/crates/v/btor2tools.svg)](https://crates.io/crates/btor2tools)
[![API](https://docs.rs/btor2tools/badge.svg)](https://docs.rs/btor2tools)
[![Lines of Code](https://tokei.rs/b1/github/ChristianMoesl/btor2tools.rs)](https://github.com/ChristianMoesl/btor2tools.rs)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ChristianMoesl/btor2tools.rs/blob/master/LICENSE)

This crate provides high-level FFI bindings for the [C Btor2Tools package](https://github.com/Boolector/btor2tools):

The Btor2Tools package provides a generic parser and tools for the BTOR2 format.

For a more detailed description of the BTOR2 format, refer to
BTOR2, BtorMC and Boolector 3.0. Aina Niemetz, Mathias Preiner, Clifford Wolf, and Armin Biere. CAV 2018.

## Status
This is work in progress. Bindings for the parser are exported and ready to use, while bindings for the simulator are not.

## Installation

This crate is on [crates.io](https://crates.io/crates/btor2tools), so you can
simply add it as a dependency in your `Cargo.toml`:
```toml
[dependencies]
btor2tools = "1"
```

This crate relies on the [`btor2tools-sys`] crate, which does statically link
C btor2tools package into your binary. So no more action required.
