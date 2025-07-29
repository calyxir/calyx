# fud2 Internals: High Level Rhai API

High level Rhai offers a convenient interface to extending fud2 by abstracting the underlying Ninja code. 

High level Rhai is relatively new and experimental, and there are certain things you cannot do in high level Rhai, but it is helpful when you want to read and write less code. Note that the Ninja code generated from high level Rhai may be difficult to read.

> We recommend reading the [The Design of fud2][fud2-design] section in the [fud2][] main page before referencing this API.

[fud2]: ./index.md
[fud2-design]: ./index.md#the-design-of-fud2

## Example Script in High Level Rhai

We'll walk through how to write a script that adds support for using the `calyx` compiler.

First, we need to define some states:

```rust,ignore
// create state named "calyx" that has associated file type "futil"
export const calyx_state = state("calyx", ["futil"]);
// create state named "verilog" that has associated file types "sv" and "v"
export const verilog_state = state("verilog", ["sv", "v"]);
```

These two lines define a `calyx` state and a `verilog` state. The first argument to `state()` is the name of the defined state, and the second argument lists file extensions that files of the state can have. The `export` prefix means that these variables will be accessible to other scripts that `import "calyx"`.

Now we will define an operation taking the `calyx` state to the `verilog` state. These operations define functions whose arguments are input files and return values are output values. Their bodies consist of commands that will transform those inputs to those outputs.

```rust,ignore
// define an op called "calyx_to_verilog" taking a "calyx_state" to a "verilog_state".
defop calyx_to_verilog(calyx_prog: calyx_state) >> verilog_prog: verilog_state {
    // retrieve a variable from the fud2.toml config
    let calyx_base = config("calyx.base");
    // retrieve a variable from the config, or a default value
    let calyx_exe = config_or("calyx.exe", `${calyx_base}/target/debug/calyx`);
    let args = config_or("calyx.args", "");

    // specify a shell command to turn a calyx file "c" into a verilog file "v"
    shell(`${calyx_exe} -l ${calyx_base} -b verilog ${args} ${calyx_prog} > ${verilog_prog}`);
}
```

Counterintuitively, `c`, `v`, `calyx_base`, `calyx_exe`, and `args` do not contain the actual variable values. They contain identifiers which are replaced by the values at runtime. For example, `print(args)` would print a `$args` instead of the value assigned by the config. **Note: In order to use Rhai variables in a command/path, you would need to put backticks around the command/path instead of quotes.** An op cannot take different code paths based on config values or different input/output file names.

This example shows off nearly all of the features available for defining ops. Scripts can reuse functionality by exploiting the tools of Rhai scripting. For example, if we wanted to create a second, similar op `calyx_noverify`, we could factor the contents of `calyx_to_verilog` into a new function and call that function in both ops. Below is an end-to-end Rhai script that does all this:

```
// create state named "calyx" that has associated file type "futil"
export const calyx_state = state("calyx", ["futil"]);
// create state named "verilog" that has associated file types "sv" and "v"
export const verilog_state = state("verilog", ["sv", "v"]);
// create state named "verilog_noverify" that has associated file types "sv" and "v"
export const verilog_noverify = state("verilog-noverify", ["sv", "v"]);

// a function constructing a shell command to take a calyx in_file to a verilog out_file
// this function adds `add_args` as extra arguments to it's call to the calyx compiler
fn calyx_cmd(in_file, out_file, add_args) {
    let calyx_base = config("calyx.base");
    let calyx_exe = config_or("calyx.exe", `${calyx_base}/target/debug/calyx`);
    let args = config_or("calyx.args", "");

    shell(`${calyx_exe} -l ${calyx_base} -b verilog ${args} ${add_args} ${in_file} > ${out_file}`);
}

// define an op called "calyx_to_verilog" taking a "calyx_state" to a "verilog_state".
defop calyx_to_verilog(calyx_prog: calyx_state) >> verilog_prog: verilog_state {
    calyx_cmd(calyx_prog, verilog_prog, "");
}

// define an op called "calyx_noverify" taking a "calyx_state" to a "verilog_noverify".
defop calyx_noverify(calyx_prog: calyx_state) >> verilog_prog: verilog_noverify {
    calyx_cmd(calyx_prog, verilog_prog, "--disable-verify");
}
```

## High Level Rhai API

### Defining states

```
state(<name>, [<ext1>, <ext2>, .. ])
```
Defines a state with the name `<name>` where files can have extensions `<ext1>`, `<ext2>`, etc. `<name>` and extensions are all strings.

### Defining operations

```
defop <op name>(<input1>: <input1 state>, <input2>: <input2 state> ...) >> <target1>: <target1 state>, <target2>: <target2 state> ... {
    <statements>
}
```
Defines an op with `<op name>` which runs `<statements>` to generate target states from input states. 

The name, inputs, and targets are called the signature of the op. `<statements>` is called the body of the op.

### Statements in operations

```
config(<config var>)
```
Returns the value of the configuration variable `<config var>` with the value provided. Panics if the configuration variable does not have a value. `<config var>` is a string.

```
config_or(<config var>, <default>)
```
Returns the value of the configuration variable `<config var>` if defined, otherwise returns `<default>`. `<config var>` and `<default>` are strings.

<br>

<div class="warning">

Each `defop` can only use *either* `shell` or `shell_deps`, not both.<br><br>If `shell` is used, Ninja will run all `shell` statements in order, preventing all parallelism/incrementalism. If `shell_deps` is used, the dependency information provided will allow parallelism & incremental builds. We recommend `shell` for simple ops.

</div>

<br>

```
shell(<string>)
```
When called in the body of an op, that op will run `<string>` as a shell command to generate its targets. It is an error to call `shell` outside of the body of an op. Additionally, it is an error to call `shell` in the body of an op in which `shell_deps` was called prior. **Note: To use any Rhai variables in the command, you should put backticks around `<string>` instead of quotes.**

In the generated Ninja code, `shell` will create both a `rule` wrapping the shell command and a `build` command that invokes that rule. When a `defop` contains multiple `shell` commands, `fud2` automatically generates Ninja dependencies among the `build` command to ensure they run in order.

```
shell_deps(<string>, [<dep1>, <dep2>, ...], [<target1>, <target2>, ..])
```
When called in the body of an op, that op will run `<string>` as a shell command if it needs to generate `<target1>, <target2>, ...` from `<dep1>, <dep2>, ...`. It is an error to call `shell_deps` outside of the body of an op. Additionally, it is an error to call `shell_deps` in the body of an op in which `shell`  was called prior. **Note: To use any Rhai variables in the command, you should put backticks around `<string>` instead of quotes.**

Targets and deps are either strings, such as `"file1"`, or identifiers, such as if `c: calyx` existed in the signature of an op then `c` could be used as a target or dep.

A call to `shell_deps` corresponds directly to a Ninja rule in the Ninja file generated by `fud2`.