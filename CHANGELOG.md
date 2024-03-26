## Current

## 0.7.0

### Language
- Added `static` abstractions for expression latency-sensitive computations.
- Deprecated `@static` attribute (#1896)
  - `@interval` attribute is used to express how quickly a component can re-execute.

### Primitives
- Reorganized `std_mem` and `seq_mem`
- `std_mem` is now called `comb_mem` and generally not preferred for use in real designs because of combinational reads
- Added new `stallable` and `pipelined` primitives. Currently, only multipliers are supported.

### Passes
- `compaction`: Pass to automatically compact control-programs based on read-write dependencies.
- `default-assigns`: New pass to add assignments for ports that have no source-level assignments.

### Tools
- `calyx-lsp`: Language server protocol implementation based on Treesitter.
- `calyx pass-help`: New command line option to provide help on passes and pass options.

### Internal
- `ReadWriteSet`: Changed to provide methods on assignments and enable chaining (#1921).


## 0.6.1
- Fix checking for large constants (#1743)
- Better static inlining for single cycle `if` (#1734)
- Implement `schedule-compaction` pass to optimize inferred static islands (#1722)
- Add `std_signext` primitive

## 0.6.0
- BREAKING: Deprecate `Cell::find_with_attr` in favor of `Cell::find_with_unique_attr`. The former is error-prone because pass logic might implicitly assume that there is only one port with a particular attribute.
- BREAKING: Redesign the `ir::Rewriter` interface to take all the rewrite maps when constructing the `ir::Rewriter` struct.
- Merge the logic of `compile-ref` pass into `compile-invoke` so that `ref` cells can be invoked.
- The `guard!` macro supports parsing complex guard expressions that use logical connectives and comparison operators.
- The `calyx` library no longer exposes any methods and should not be depended upon. Instead, the new `calyx-backend` crate provides the code needed to emit Verilog from Calyx.

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