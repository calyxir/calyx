# fud2 Internals: Rhai API

[fud2][] is the compiler driver tool for orchestrating the Calyx ecosystem. Its functionality is written using [Rhai][rhai], a scripting language that is tightly connected with Rust. The Rhai code is used to emit [Ninja][ninja] files which specify the build process.

fud2 Rhai code can be written in two "level"s: High Level Rhai and Low Level Rhai. High Level Rhai is convenient for writing code in a more declarative way. Low Level Rhai code looks closer to the emitted Ninja code, allowing users to have more control. **Check out [High Level Rhai][high-level-rhai] and [Low Level Rhai][low-level-rhai] for examples code and API documentation! This page contains general Rhai API information.**

All existing fud2 Rhai scripts can be found [here][fud2-scripts]. These provide a good example of how to add states and operations with Rhai. This page describes the fud2 Rhai API for both people who are looking to use fud2 and extend the functionality of fud2.

## Extending fud2 functionality with a new Rhai script

There are two ways to add new functionality to fud2:

- Add a new Rhai script to `<CALYX_BASE_DIR>/fud2/scripts`
- Load a script to `fud2` by editing the configuration file (`fud2 edit-config`):
```toml
plugins = ["/my/fancy/plugin.rhai"]

[calyx]
base = "..."
```

## Adding to the API

If there is something that is hard to do in Rhai, it is straightforward to [register a Rust function][rhai-rust-fn] so that it is available from Rhai.

Rust functions are registered in [`ScriptRunner::new`][fud-core-scriptrunner]. Refer to [`ScriptRunner::reg_get_state`][fud-core-reg_get_state] to see a simple example of how to register a function.

## Rhai Specifics

### String Templates

Rhai has a string templating feature, similar to the `format!` macro in rust. Templated strings are marked with backticks (`` `path/${some_var}.ext` ``) and variables are included with `$`. You can include expressions that will be evaluated by using brackets: `${1 + 2}`.

### String Functions

Rhai includes standard string operations. They are described in the [documentation][rhai-strings]. These are useful for constructing more complicated paths.

### Export Rules

In Rhai, all top-level variable declarations are private by default. If you want them to be available from other files, you need to `export` them explicitly.

All functions are exported by default. However, they are only exported in a callable format. If you want to use the function as a variable (when passing them as a setup function or build function), you need to export them explicitly as well.

This is how that looks:
```rust,ignore
export const my_fancy_setup = my_fancy_setup;
fn my_fancy_setup(e) {
   ...
}
```

### Imports

You can import another Rhai script file like so:

```rust,ignore
import "calyx" as c;
```

All exported symbols defined in `calyx.rhai` will be available under `c`.

```rust,ignore
print(c::calyx_state);
print(c::calyx_setup);
```

<div class="warning">

The name for an import is always just the basename of the script file, without any extension.

</div>

[fud2]: ./index.md
[rhai]: https://rhai.rs/book/index.html
[ninja]: https://ninja-build.org
[rhai-strings]: https://rhai.rs/book/ref/string-fn.html?highlight=String#standard-string-functions
[rhai-rust-fn]: https://rhai.rs/book/rust/functions.html
[fud2-scripts]: https://github.com/calyxir/calyx/tree/main/fud2/scripts
[fud-core-scriptrunner]: https://github.com/calyxir/calyx/blob/6f895a1353020ce254860c3aa0fcfa2ba1abf4c4/fud2/fud-core/src/script/plugin.rs#L68
[fud-core-reg_get_state]: https://github.com/calyxir/calyx/blob/6f895a1353020ce254860c3aa0fcfa2ba1abf4c4/fud2/fud-core/src/script/plugin.rs#L152
[high-level-rhai]: ./high-level-rhai.md
[low-level-rhai]: ./low-level-rhai.md