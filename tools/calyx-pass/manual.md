# User Manual

## Command Line

`calyx-pass` takes a single filename as input.
This file must be a valid calyx program.

### `-c, --component <name>`

HIGHLY RECOMMENDED.
Restricts the output to a particular component.

### `-e, --calyx-exec <path>`

You may provide `calyx-pass` with the path to the `calyx` driver executable.
If you don't, it will first use `fud`'s (the original) configuration, and if that fails, it will assume you are running `calyx-pass` from the root directory of the repository.

### `-b, --break <pass>`

Sets a breakpoint at `pass` by accepting all passes up until `pass`.

#### `-d, --disable-pass <pass2...>`

As an exception to the previous sentence, if `-d <pass2>` is provided along with a breakpoint, then `pass2` will be skipped if it occurs before the breakpoint `pass`.

## App

The explorer interface responds to key commands in two main categories: Analysis and Movement.

### Analysis

| Command | Description |
| --- | ----------- |
| `a` | Accepts incoming pass |
| `s` | Skips incoming pass |
| `u` | Undoes the last accept/skip |
| `q` | Exits the program |

Please do not use CTRL-C to exit.

### Movement

The arrow keys and scrolling works as normal.
In addition, `f` and `b` jump forward and backward by 4 lines.
