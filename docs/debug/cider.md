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

CIDR supports breakpoints on group definitions. This helps focus attention on
suspect portions of the code.

### Setting a breakpoint

Breakpoints may be set on the main component by simple specifying the group of
interest.

```
 > break group_1
```

This is identical to

```
 > break main::group_1
```

For sub-components, the name of the sub-component must be included with the
double colon separating the group name. To break on the `do_mul` group inside
the `pow` sub-component:

```
 > break pow::do_mul
```

### Managing breakpoints

To see a list of breakpoints:
```
 > info break
```
or
```
 > ib
```

This produces output like this:
```
 > ib
     Current breakpoints:
    1.  main::group_1  enabled
    2.  pow::do_mul enabled
```

All breakpoints have a number associated with them and they may be managed with
this number or the group name.

To enable or disable a breakpoint:

```
 > disable group_1 2
 > enable 1 pow::do_mul
```

Note that this is equivalent to:
```
 > disable group_1
 > disable 2
 > enable 1
 > enable pow::do_mul
```

To delete a breakpoint:
```
 > delete 1
 > del pow::do_mul
```

Deleted breakpoints will be entirely removed while disabled breakpoints will
remain until they are either enabled again or subsequently deleted. Disabled
breakpoints will not cause program execution to halt when `continue`-ing.

## Inspecting State

### `display`

The display command dumps the full state of the main component without
formatting. Use the `print` and `print-state` commands for targeted inspection
with formatting.

### Formatting codes

CIDR supports several different formatting codes which do the hard work of
interpreting the data in human readable ways.


| name | code | description
|------|------|-----------|
|binary|  | The default, a bit vector with the msb on the left
|unsigned| \u | Unsigned bit-num formatting
|signed| \s | Two's Complement formatting
|unsigned fixedpoint| \u.N | For N >=1. Unsigned Fixed-point with N fractional bits. The remaining bits are for the integral component.
|signed fixedpoint| \s.N | For N >=1. Signed Fixed-point with N fractional bits. The remaining bits are for the integral component.

### `print` and `print-state`

These commands allow inspecting *instance* state with optional formatting. Note
that this is different from breakpoints which operate on *definitions*. For example to print the ports of the `std_mul` instance named `mul` in the `pow` instance `pow_1` attached to the main component:

```
 > print main.pow_1.mul
```

as with breakpoints, the leading `main` may be elided:
```
 > print pow_1.mul
```

This will print all the ports attached to this multiplier instance with binary
formatting.

Formatting codes may be supplied as the first argument.

```
 > print \u pow_1.mul
```

The `print` may also target specific ports on cells, rather than just the cell
itself. To see only the output of the multiplier (with unsigned formatting):

```
 > print \u pow_1.mul.out
```

The `print-state` command works in the same way as the `print` command, except
it displays the internal state of a cell, rather than port values. As such, it
can only target cells and only those with some internal state, such as registers
or memories. For example, if the main component has a memory named `out_mem` its
contents may be viewed via:

```
 > print-state main.out_mem
```

or just

```
 > print-state out_mem
```

As with `print`, `print-state` supports formatting codes as an optional first
argument. So to view the contents of `out_mem` with a signed interpretation:

```
 > print-state \s out_mem
```

## Watchpoints

Watchpoints are like breakpoints but rather than stop the execution when they
are passed, they instead print out some information. Like breakpoints, they are
set on group *definitions*, such as `main::group_1` or `pow::do_mul`

### Setting watchpoints

The general form of watchpoints looks like
```
watch [POSITION] GROUP with PRINT-COMMAND
```

where:

- `GROUP` is the group definition to be watched
- `PRINT-COMMAND` is a full `print` or `print-state` command to be run by the watchpoint

The optional `POSITION` argument may either be `before` or `after`. This
specifies whether the watchpoint should run when the group first becomes active
(`before`) or when the group finishes running (`after`). This defaults to
`before` if not set.

### Managing watchpoints

Watchpoint management is similar to breakpoints. However there may be multiple
watchpoints for a single group definition, so deleting watchpoints via the group
name will delete all the watchpoints associated with the group. Watchpoints do
not currently have an enable/disable state.

To view all the watchpoint definitions:

```
 > info watch

...

 > iw
```

To delete watchpoints:

```
 > delete-watch 1
 > del-watch main::group_1
```

## Viewing the program counter

There `where` command (alias `pc`) displays the currently running portion of the
control tree including active subcomponents. This can be used to more easily
determine the currently active portion of the design as well as visualize how
much of the execution is occurring in parallel at any given point.

## Exiting the debugger

Use `help` to see all commands. Use `exit` to exit the debugger.


[fud]: ../running-calyx/fud/index.md
[gdb]: https://sourceware.org/gdb/
[interp]: ../interpreter.md
