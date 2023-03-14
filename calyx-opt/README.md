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

The `calyx` crate contains the Rust implementation of the intermediate
representation, the compiler passes, and a frontend to parse source programs
into the intermediate representation.

If you'd like try out the compiler infrastructure, take a look at the
[`futil`][futil] crate instead.

[site]: https://calyxir.org
[docs]: https://docs.calyxir.org
[source-docs]: https://docs.calyxir.org/source/calyx/
[paper]: https://rachitnigam.com/files/pubs/calyx.pdf
[futil]: https://crates.io/crates/futil
