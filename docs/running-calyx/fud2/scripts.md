# Scripting `fud2` with `Rhai`

You can add functionality to `fud2` with functionality written in [Rhai][rhai]. Rhai is a scripting language designed to work well with Rust.

All functionality included with `fud2` is written in Rhai. They can be found [here][fud2-scripts]. These provide a good example of how to add states and operations with Rhai. 

## Loading Scripts

You can tell `fud2` to load a script by including a `plugins` key in your `fud2.toml` file.

```toml
plugins = ["/my/fancy/plugin.rhai"]

[calyx]
base = "..."
```

## Example Script in High Level Rhai

We'll walk through how to write a script that adds support for using the `calyx` compiler.

First, we need to define some states:

```rust,ignore
export const calyx_state = state("calyx", ["futil"]);
export const verilog_state = state("verilog", ["sv", "v"]);
```

These two lines define a `calyx` state and a `verilog` state. The `export` prefix means that these variables will be accessible to other scripts that `import "calyx"`.

Now we will define an operation taking the `calyx` state to the `verilog` state. These operations define functions whose arguments are input files and return values are output values. Their bodies consist of commands that will transform those inputs to those outputs.

```rust,ignore
// define an op called "calyx_to_verilog" taking a "calyx_state" to a "verilog_state".
defop calyx_to_verilog(c: calyx_state) >> v: verilog_state {
    // retrieve a variable from the fud2.toml config
    let calyx_base = config("calyx.base");
    // retrieve a variable from the config, or a default value
    let calyx_exe = config_or("calyx.exe", "${calyx_base}/target/debug/calyx");
    let args = config_or("calyx.args", "");

    // specify a shell command to turn a calyx file "c" into a verilog file "v"
    shell("${calyx_exe} -l ${calyx_base} -b verilog ${args} ${c} > ${v});
}
```

Counterintuitively, `c`, `v`, `calyx_base`, `calyx_exe`, and `args` do not contain the actual variable values. They contain identifiers which are replaced by the values at runtime. For example, `print(args)` would print a `$args` instead of the value assigned by the config. An op cannot take different code paths based on config values or different input/output file names.

This example shows off nearly all of the features available for defining ops. Scripts can reuse functionality by exploiting the tools of Rhai scripting. For example, if we wanted to create a second, similar op `calyx_noverify`, we could factor the contents of `calyx_to_verilog` into a new function and call that function in both ops.

```
// a function constructing a shell command to take a calyx in_file to a verilog out_file
// this function adds `add_args` as extra arguments to it's call to the calyx compiler
fn calyx_cmd(in_file, out_file, add_args) {
    let calyx_base = config("calyx.base");
    let calyx_exe = config_or("calyx.exe", "${calyx_base}/target/debug/calyx");
    let args = config_or("calyx.args", "");

    shell("${calyx_exe} -l ${calyx_base} -b verilog ${args} ${add_args} ${in_file} > ${out_file});
}

// define an op called "calyx_to_verilog" taking a "calyx_state" to a "verilog_state".
defop calyx_to_verilog(c: calyx_state) >> v: verilog_state {
    calyx_cmd(c, v, "");
}

// define an op called "calyx_noverify" taking a "calyx_state" to a "verilog_state".
defop calyx_to_verilog(c: calyx_state) >> v: verilog_state {
    calyx_cmd(c, v, "--disable-verify");
}
```

## Example Script in Low Level Rhai

`fud2` generates Ninja build files. Low level Rhai gives more control over what generated build files look like.

We'll walk through how to write a script that adds support for using the `calyx` compiler.

Like before, we need to define some states:

```rust,ignore
export const calyx_state = state("calyx", ["futil"]);
export const verilog_state = state("verilog", ["sv", "v"]);
```

These two lines define a `calyx` state and a `verilog` state. The `export` prefix means that these variables will be accessible to other scripts that `import "calyx"`.

Next we'll define a setup procedure to define some rules that will be useful.

```rust,ignore
// allows calyx_setup to be used in other scripts
export const calyx_setup = calyx_setup;

// a setup function is just a normal Rhai function that takes in an emitter
// we can use the emitter in the same way that we use it from rust
fn calyx_setup(e) {
   // define a Ninja var from the fud2.toml config
   e.config_var("calyx-base", "calyx.base");
   // define a Ninja var from either the config, or a default derived from calyx-base
   e.config_var_or("calyx-exe", "calyx.exe", "$calyx-base/target/debug/calyx");
   // define a Ninja var from cli options, or with a default
   e.config_var_or("args", "calyx.args", "");
   // define a rule to run the Calyx compiler
   e.rule("calyx", "$calyx-exe -l $calyx-base -b $backend $args $in > $out");
}
```

And now we can define the actual operation that will transform `calyx` files into `verilog` files.

```rust,ignore
op(
  "calyx-to-verilog",      // operation name
  [calyx_setup],           // required setup functions
  calyx_state,             // input state
  verilog_state,           // output state
  |e, input, output| {     // function to construct Ninja build command
    e.build_cmd([output], "calyx", [input], []) ;
    e.arg("backend", "verilog");
  }
);
```

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