# fud2: An Experimental Successor to fud

[fud][] is the compiler driver tool for orchestrating the Calyx ecosystem.
fud2 is an experiment in building a new driver that works like fud that adds some fundamental new capabilities and resolves some underlying problems.

"Original" fud is still the right tool for almost all jobs; fud2 is in an experimental phase and does not support everything fud can do.
Someday, fud2 may supplant fud, but it needs more work before it is ready to do that.
Until then, fud remains your first choice for all your build-related needs.

[fud]: ./fud/index.md

## Set Up

fud2 is a Rust tool, so you can build it along with everything else in this monorepo with `cargo build`.
You might then want to do something like ``ln -s `pwd`/target/debug/fud2 ~/.local/bin`` for easy access to the `fud2` binary.

fud2 depends on [Ninja][].
Install it using your OS package manager or by downloading a binary.

Create a configuration file at `~/.config/fud2.toml`, using the path to your checkout of the Calyx git repository:

```toml
rsrc = ".../calyx/fud2/rsrc"

[calyx]
base = ".../calyx"
```

Now you're ready to use fud2.

[ninja]: https://ninja-build.org

## General Use

You can see complete command-line documentation with `fud2 --help`.
But generally, you want to do something like this:

    $ fud2 <IN> -o <OUT>

For example, use this to compile a Calyx program to Verilog:

    $ fud2 foo.futil -o bar.sv

fud2 tries to automatically guess the input and output formats using filename extensions.
If that doesn't work, you can choose for it with `--from <state>` and `--to <state>`;
for example, this is a more explicit version of the above:

    $ fud2 foo.futil -o bar.sv --from calyx --to verilog

You can also omit the input and output filenames to instead use stdin and stdout.
In that case, `--from` and `--to` respectively are required.
So here's yet another way to do the same thing:

    $ fud2 --from calyx --to verilog < foo.futil > bar.sv

This is handy if you just want to print the result of a build to the console:

    $ fud2 foo.futil --to verilog

Some operations use other configuration options, which can come from either your `fud2.toml` or the command line.
Use `--set key=value` to override any such option.

## Advanced Options

Here are some options you might need:

* By default, fud2 runs the build in a directory called `.fud2` within the working directory. It automatically deletes this directory when the build is done.
    * It can be useful to keep this build directory around for debugging or as a "cache" for future builds. Use `--keep` to prevent fud2 from deleting the build directory.
    * You can also tell fud2 to use a different build directory with `--dir`. If you give it an existing directory, it will never be deleted, even without `--keep`. (Only "fresh" build directories are automatically cleaned up.)
* If you don't like the operation path that fud2 selected for your build, you can control it with `--through <OP>`. fud2 will search the operation graph for a path that contains that op. You can provide this option multiple times; fud2 will look for paths that contain *all* these operations, in order.
* You can choose one of several modes with `-m <NAME>`:
    * `run`: Actually execute a build. The default.
    * `gen`: Generate the Ninja build file in the build directory, but don't actually run the build. The default `run` mode is therefore approximately like doing `fud2 -m gen && ninja -C .fud2`.
    * `emit`: Just print the Ninja build file to stdout. The `gen` mode is therefore approximately `fud2 -m emit > .fud2/build.ninja`.
    * `plan`: Print a brief description of the plan, i.e., the sequence of operations that the build would run.
    * `dot`: Print a [GraphViz][] depiction of the plan. Try `fud2 -m dot | dot -Tpdf > graph.pdf` and take a look.

[graphviz]: https://graphviz.org
