# calyx-pass

`calyx-pass` (name TBD) is a work-in-progress *pass transformation* explorer for calyx.
You give it an input file and some options, and you can explore 

![Example running of the tool](example_v0.0.0.png)

## Features

- Visualize pass transformations with readable diff insertions. By default, it uses `-p all`, but you can provide a different alias.
- Focus on changes localized to one specific component with the `-c` option.
- Set a breakpoint for pass exploration with the `-b` option. You can then use `u` to undo previous passes and explore the changes before and after the pass.
- Nearly-arbitrary pass execution with TUI commands. Currently `a`/`s`/`u` (accept/skip/undo) are supported, and `r` (run given pass) is [planned](TODO.md).
- And all those unchecked in [TODO.md](TODO.md) are coming soon!

## Usage

Pass `--help` to view invocation information.
The TUI (terminal user interface, like a GUI but with text) displayed when running the program has instructions for how to use it.
If you do not pass `-e`/`--calyx-exec` with the path to the `calyx` driver executable, it will infer it as follows:

1. It will try to use `fud`'s configuration to piece together the location.
2. It will assume you are running it from the calyx root directory having already ran `cargo build`.

## Development

Run `make dev` before starting development and run `make undev` before committing.

## Known Issues

The program currently works by maintaining files and not `calyx_ir` objects, so the pretty-print will produce internal symbols that cannot be reparsed. For example, running `-p simplify-with-control` after `-p cell share` leads to such an issue:

```
Error: Failed to parse `test2.futil`:   --> 83:5
   |
83 |     @NODE_ID(0) seq {
   |     ^---------^
   |
   = Invalid attribute: NODE_ID. All caps attributes are reserved for internal use.
```

## Author

This tool was designed and developed by [Ethan Uppal](https://www.ethanuppal.com).
