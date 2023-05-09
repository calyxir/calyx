<h1>
<p align="center">
<img src="https://calyxir.org/img/logo-text.svg" width="300">
</p>
<p align="center">
<a href="https://calyxir.org">A Compiler Infrastructure for Accelerator Generators</a>
</p>
</h1>

Calyx is an intermediate language and infrastructure for building compilers that generate custom hardware accelerators.

See the [Calyx website][site], [language documentation][docs] and the
[documentation for the source code][source-docs]
for more information. Calyx's design is based on [our paper][paper].

## Installation

### Quick
If you want to try out the compiler, install it using `cargo`:
```
cargo install calyx
```

This will install the `futil` binary which includes the calyx frontend,
optimization passes, and several backends.

### Recommended

Follow the [getting started][docs] instructions.

## Organization

This repository contains the source code for the following:
1. [`calyx`][] (`calyx/`): The intermediate representation used for hardware
   accelerator generation.
2. [`futil`][] (`src/`): The compiler infrastructure for compiling Calyx programs.
   If `calyx` is like LLVM, then `futil` is Clang.
3. Calyx debugger (`interp/`): An interpreter and debugger for Calyx.
4. `fud`, The Calyx driver: Utility tool that wraps various hardware toolchains.

[site]: https://calyxir.org
[docs]: https://docs.calyxir.org
[source-docs]: https://docs.calyxir.org/source/calyx
[paper]: https://rachitnigam.com/files/pubs/calyx.pdf
[`calyx`]: https://crates.io/crates/calyx
[`futil`]: https://crates.io/crates/futil
