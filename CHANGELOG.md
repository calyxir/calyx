## Unreleased
- BREAKING: Deprecate `Cell::find_with_attr` in favor of `Cell::find_with_unique_attr`. The former is error-prone because pass logic might implicitly assume that there is only one port with a particular attribute.
- BREAKING: Redesign the `ir::Rewriter` interface to take all the rewrite maps when constructing the `ir::Rewriter` struct.
- Merge the logic of `compile-ref` pass into `compile-invoke` so that `ref` cells can be invoked.

## 0.5.1
- Change the `calyx` build script to use the `CALYX_PRIMITIVES_DIR` env variable to install primitive libraries. If unset, use `$HOME/.calyx`.

## 0.5.0
- Don't require `@clk` and `@reset` ports in `comb` components
- `inline` pass supports inlining `ref` cells
- `comb-prop`: disable rewrite from `wire.in = port` when the output of a wire is read.
- BREAKING: Remove `PortDef::into()` because it makes it easy to miss copying attributes.
- Remove the `futil` binary.
- The `calyx` binary ships all the primitives and therefore self-contained now.
    - Add the `calyx-stdlib` package
    - Add a new build script that installs primitives when the package is installed.


## 0.4.0
- Language: New `repeat` operator that can be used in dynamic contexts as well. When possible, `static-promotion` will attempt to promote it.
- Fix: `wrap-main` correctly instantiates the original `"toplevel"` component in the generated `main` component.
- Make `Workspace::construct_with_all_deps` public to allow construction of multi-file workspaces.
- Don't emit `clk` ports for `@external` cells in the AXI generator.
- BREAKING: Redesign the interface for `LibrarySignatures`.
    - Expose methods to add new primitives to the library
    - Rewrite the IR printer to print out source primitives when `skip_primitive` is set.


## 0.3.0
- `ir::Component` takes a `has_interface` argument and ensures that interface ports are present when it is true.
- The `Visitor` trait supports new `start_context` and `finish_context` methods which allow the pass to affect the context before and after the components are visited respectively.
- New `wrap-main` pass that generates a top-level `main` component if the top-level component is not named that.
- Pretty printer prints code more tersely.

## 0.2.1
- Remove necessary indentation inlined verilog primitives
- Add new `discover-external` pass to transform inlined cells into `@external` cells
- Implementation of `static` primitives and components and finish work on static milestone paving way for deprecation of the `@static` attribute.
- Get rid of generation of `initial` blocks and the `--disable-init` flag.

## 0.2.0
- The core compilation primitives are included in the compiler distribution eliminating reliance on the primitives path.
- Add new `data-path-infer` pass to infer `@data` annotation allowing more backend optimization
- Distribute `calyx` and `futil` binaries together and show deprecation warning for the latter.

## 0.1.0
- Initial release with seperate Calyx crates and the new `static` primitives