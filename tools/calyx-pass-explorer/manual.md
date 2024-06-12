# User Manual

## Command Line

`calyx-pass` takes a single filename as input.
This file must be a valid calyx program.

| Option | Argument(s) | Description |
|--------|-------------|-------------|
| `-c, --component` | `<name>` | **HIGHLY RECOMMENDED**. Restricts the output to a particular component. |
| `-e, --calyx-exec` | `<path>` | You may provide `calyx-pass` with the path to the `calyx` driver executable. If you don't, it will first use (the original) `fud`'s configuration, and if that fails, it will assume you are running `calyx-pass` from the root directory of the repository. |
| `-b, --break` | `<pass>` | Sets a breakpoint at `pass` by accepting all passes up until `pass`. |
| `-d, --disable-pass` | `<pass2...>` | As an exception to the previous sentence, if `-d <pass2>` is provided along with a breakpoint, then `pass2` will be skipped if it occurs before the breakpoint `pass`. |

## TUI App

The explorer interface responds to key commands in two categories.

### Exploration

| Command | Description |
| --- | ----------- |
| `a` | Accepts incoming pass |
| `s` | Skips incoming pass |
| `u` | Undoes the last accept/skip |
| `q` | Exits the program |

Please do not use CTRL-C to exit.
That will forcefully terminate the program, preventing correct cleanup.

### Movement

The arrow keys and scrolling (if enabled in your terminal emulator) work as normal.
In addition, the `f` and `b` keys jump forward and backward by `tui::JUMP` lines.
