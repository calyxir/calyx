## 0.2.1
- Remove necessary indentation inlined verilog primitives
- Add new `discover-external` pass to transform inlined cells into `@external` cells
- Implementation of `static` primitives and components and finish work on static milestone paving way for deprecation of the `@static` attribute.

## 0.2.0
- The core compilation primitives are included in the compiler distribution eliminating reliance on the primitives path.
- Add new `data-path-infer` pass to infer `@data` annotation allowing more backend optimization
- Distribute `calyx` and `futil` binaries together and show deprecation warning for the latter.

## 0.1.0
- Initial release with seperate Calyx crates and the new `static` primitives