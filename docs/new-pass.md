# Adding a New Pass

All passes in the compiler are stored in the `calyx/src/passes` directory.
To add a new pass, we need to do a couple of things:
1. Define a pass struct and implement the required traits.
2. Expose the pass using in the `passes` module.
3. Register the pass in the compiler.

> It is possible to add passes outside the compiler tree, but we haven't needed to do this yet, so we will not cover it here.

## Defining a Pass Struct

We first define a [Rust structure][struct] that will manage the state of the pass:
```rust
pub struct NewPass;
```

A pass needs to implement the [`Named`][named-trait] and [`Visitor`][visitor-trait] traits.
The former defines the name, description, and [pass-specific options][pass-opts] of the pass.

```rust
impl Named for NewPass {
    fn name(&self) -> &'static str { "new-pass" }
    ...
}
impl Visitor for NewPass { ... }
```

The pass name provided in used in the compiler's driver and needs to be unique for each registered pass.

## The Visitor Trait

The visitor trait allows us to define the behavior of the pass.
The visitor visits each [control operator][control] in each component and performs some action.
Furthermore, it also allows us to control the order in which components are visited.

### Component Iteration Order

The [`Order`][order] struct allows us to control the order in which components are visited:
- `Post`: Iterate the subcomponents of a component before the component itself.
- `Pre`: Iterate the subcomponents of a component after the component itself.
- `No`: Iterate the components in any order.

### Visiting Components

Most passes will attempt to transform the structural part of the program (`wires` or `cells`), the `control` schedule, or both.
The `Visitor` trait is flexible enough to allow all of these patterns and efficiently traverse the program.

For a control program like this:
```
seq {
    one;
    if cond { two } else { three }
    invoke foo(..)
}
```

The following sequence of `Visitor` methods are called:
```
- start
- start_seq
  - enable       // group one
  - start_if
    - enable     // group two
    - enable     // group three
  - end_if
  - invoke       // invocation
- finish_seq
- finish
```

Each non-leaf control operator defines both a `start_*` and `finish_*` method which allows us to encode top-down and bottom-up traversal patterns.

Each method returns an [`Action`][action] value which allows us to control the traversal of the program.
For example, `Action::Stop` will immediately stop the traversal of the program while `Action::SkipChildren` will skip the traversal of the children of the current control operator.

## Registering the Pass

The final step is to register the pass in the compiler.
We use the [`PassManager`][pass-manager] to register the pass defined in the `default_passes.rs` file.

Registering a pass is as simple as calling the register pass:
```rust
pm.register_pass::<NewPass>();
```

Once done, the pass is accessible from the command line:
```bash
cargo run -- -p new-pass <file>
```

This will run `-p new-pass` on the input file.
In order to run this pass in the default pipeline, we need to add it to the `all` alias (which is called when no `-p` option is provided).
The `all` alias is itself defined using other aliases which separate the pipeline into different phases.
For example, if `NewPass` needs to run before the compilation passes, we can add it to the `pre-opt` alias.

## Some Useful Links

The compiler has a ton of shared infrastructure that can be useful:
- [`ir::Context`][context]: The top-level data structure that holds a complete Calyx program.
- [Rewriter][]: Helps with consistent renaming of ports, cells, groups, and comb groups in a component.
- [`analysis`][analysis]: Provides a number of useful analysis that can be used within a pass.
- IR macros: Macros useful for adding cells ([`structure!`][structure]), guards ([`guard!`][guard]) and assignments ([`build_assignments!`][build-assigns]) to component.

[pass-opts]: ./compiler.md#providing-pass-options
[control]: ./lang/ref.md#the-control-operators
[struct]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html
[named-trait]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/traversal/trait.Named.html
[visitor-trait]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/traversal/trait.Visitor.html
[pass-manager]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/pass_manager/struct.PassManager.html
[action]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/traversal/enum.Action.html
[order]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/traversal/enum.Order.html
[rewriter]: https://docs.rs/calyx-ir/latest/calyx_ir/rewriter/index.html
[analysis]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/analysis/index.html
[build-assigns]: https://docs.rs/calyx-ir/latest/calyx_ir/macro.build_assignments.html
[guard]: https://docs.rs/calyx-ir/latest/calyx_ir/macro.guard.html
[structure]: https://docs.rs/calyx-ir/latest/calyx_ir/macro.structure.html
[context]: https://docs.rs/calyx-ir/latest/calyx_ir/struct.Context.html