<h1>
<p align="center">
<img src="https://capra.cs.cornell.edu/calyx/img/logo-text.svg" width="300">
</p>
<p align="center">
<a href="https://capra.cs.cornell.edu/calyx">A Compiler Infrastructure for Accelerator Generators</a>
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

[site]: https://capra.cs.cornell.edu/calyx
[docs]: https://capra.cs.cornell.edu/docs/calyx/
[source-docs]: https://capra.cs.cornell.edu/docs/calyx/source/calyx
[paper]: https://rachitnigam.com/files/pubs/calyx.pdf
[futil]: https://crates.io/crates/futil
