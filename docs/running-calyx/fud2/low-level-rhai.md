# Low Level Rhai

`fud2` generates Ninja build files. Low level Rhai gives more control over what generated build files look like.

> We recommend reading the [The Design of fud2][fud2-design] section in the [fud2][] main page before referencing this API.

[fud2]: ./index.md
[fud2-design]: ./index.md#the-design-of-fud2

## Example Script in Low Level Rhai

A common routine for using low level Rhai is:
1. Define states
2. Define "setup" functions that register variables and rules
3. Define a "builder" function that produces commands using variables and rules created from (2)
4. Define an op that uses "setup" functions and the "builder" function to produce the target state from the start state

"Setup" and "builder" functions are normal Rhai functions that use the Emitter API.

We'll walk through how to write a script that adds support for using the `calyx` compiler.

Like in High Level Rhai, we need to define some states:

```rust,ignore
export const calyx_state = state("calyx", ["futil"]);
export const verilog_state = state("verilog", ["sv", "v"]);
```

These two lines define a `calyx` state and a `verilog` state. The `export` prefix means that these variables will be accessible to other scripts that `import "calyx"`.

Next we'll define a "setup" procedure to define some rules that will be useful.

```rust,ignore
// allows calyx_setup to be used in other scripts
export const calyx_setup = calyx_setup;

// a "setup" function is just a normal Rhai function that takes in an emitter
// defines Ninja vars and rules
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
  [calyx_setup],           // required "setup" functions
  calyx_state,             // input state
  verilog_state,           // output state
  |e, input, output| {     // "build" function - constructs Ninja build command
    e.build_cmd([output], "calyx", [input], []) ;
    e.arg("backend", "verilog");
  }
);
```

## Low Level Rhai API

### Defining states

```
state(<name>, [<ext1>, <ext2>, .. ])
```
Defines a state with the name `<name>` where files can have extensions `<ext1>`, `<ext2>`, etc. `<name>` and extensions are all strings.

### Defining operations

```
op(<op name>, [<setup1>, <setup2>, ..], <input state>, <output state>, <emit function>)
```

Defines an op with name `<op name>` that generates `<output state>` from `<input state>`. `<op name>` is a string, and `<input state>` and `<output state>` are states. 

The op uses the setup functions `<setup1>, <setup2>, ..` to create vars and rules, and the `<emit function>` to produce commands.

A "setup" function is simply a function that takes in an emitter. It usually calls functions that [create variables](#creating-variables) and [define rules](#defining-rules).

The "builder" function would have the following declaration:
```
|e, input, output| { ... }
```
where `e` is an emitter, and `input` and `output` are strings containing the filenames of the input and output of the operation. It usually calls functions that [build commands](#building-commands).

### Emitter API

Functions listed in this section are methods of the Emitter, which can be called to create variables, rules, and commands in the produced Ninja file. 

#### Creating variables

```
var_(<variable name>, <filename>)
```
Defines a Ninja variable `<variable name>` which refers to the file `<filename>`. Both `<variable name>` and `<filename>` are strings.

<br>

```
config_var(<variable name>, <configuration path>)
```
Defines a Ninja variable `<variable name>` using the value obtained from `<configuration path>` from the configuration file. If the config value is undefined, raises an error. Both `<variable name>` and `<configuration path>` are strings.

<br>

```
config_var_or(<variable name>, <configuration path>, <default>)
```
Defines a Ninja variable `<variable name>` using the value obtained from `<configuration path>` from the configuration file if the value exists, or `<default>` otherwise. Both `<variable name>` and `<configuration path>` are strings.

<br>

```
config_val(<config var>)
```
Returns the value of the configuration variable `<config var>` with the value provided. Panics if the configuration variable does not have a value.

<br>

```
config_constrained_or(<config var>, [<valid val1>, <valid val2>, ..], <default>)
```
Returns the value of the configuration variable `<config var>` if the value provided is valid, i.e. contained within `[<valid val1>, <valid val2>, ..]`. Panics if the value is not valid.

If configuration variable `<config var>` was not provided, then `<default>` will be returned.

`<config var>`, all `<valid val>`s, and `<default>` are all strings.

<br>

```
external_path(<path>)
```
Returns a `Utf8PathBuf` to an external file described by `<path>`. The input `path` may be relative to our original invocation; we make it relative to the build directory so it can safely be used in the Ninja file.


#### Defining rules

```
rule(<rule name>, <shell command>)
```
Registers a shell command `<shell command>` under the Ninja rule name `<rule name>`. Both `<rule name>` and `<shell command>` are strings.

#### Building commands

```
build_cmd([<target>], <rule>, [<dep1>, <dep2>, ..], [<implicit_dep1>, <implicit_dep2>, ..])
```
Emits a Ninja build command that generates the build target `<target>` with the rule `<rule>` by using `<dep1>, ..` and `<implicit_dep1>, ..`.

`<target>` replaces `$out` in `<rule>`, and `<dep1>` replaces `$in` in `<rule>` (if there are multiple dependencies, then a string containg all dependencies with spaces in between is passed in). `<implicit_dep1>, ..` should be strings containing variable names defined previously defined via setups referred as `$<var name>`.

<br>

```
arg(<arg name>, <value>)
```
Adds the argument `<arg name>` to the preceding rule or build command with the value `<value>`.