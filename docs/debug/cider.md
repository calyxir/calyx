# The Calyx Interactive Debugger

The Calyx Interactive Debugger is a prototype debugging tool built on top of the
[Calyx Interpreter][interp] which exposes a [gdb][gdb]-like interface for
debugging Calyx programs.

## Getting Started

If you are using [`fud`][fud] getting started with the debugger is easy.
Assuming you are trying to debug a program called `my_program.futil` with data
file `my_program.futil.data`, invoke the debugger with the following command:

```
fud e --to debugger -q my_program.futil -s verilog.data my_program.futil.data
```

This will open the target program in the interactive debugger. Note that `fud`
uses **the quiet flag**, `-q`, here. This prevents the printing from the `fud` tool
from conflicting the debugger as both tools interact with standard out.

## Advancing Program execution


### `step`

The simplest way to advance the program is via the `step` command which causes
time to advance by a clock tick. It also has a shortcode: `s`.

```
 > step
 > s
```

The above snippet advances the program by two steps.

### `step-over`

Another way to advance the program is via the `step-over` command. Unlike the
`step` command, this command requires a second argument which is the name of the
group to advance over. The `step-over` command then advances the program until
the given group is no longer running.

If you want to use the command to advance the program past a group `group_1`, do
the following:

```
 > step-over group_1
```

Note that the `step-over` command will do nothing if the given group is not
running.

```
 > step-over other_group
 Group is not running
 >
```

### `continue`

Finally, the continue command will run the program until either a breakpoint is
hit or the program terminates. This command is used in conjunction with
breakpoints and watchpoints to provide more targeted inspection. It may also be
accessed with the shortcode `c`.

```
 > continue
Main component has finished executing. Debugger is now in inspection mode.
```

## Breakpoints

[fud]: /fud/index.md
