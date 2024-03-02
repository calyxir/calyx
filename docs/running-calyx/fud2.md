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

### Configuration

Run the following command to edit `fud2`'s configuration file (usually `~/.config/fud2.toml`):

    $ fud2 edit-config

Add these lines:

```toml
[calyx]
base = "<path to calyx checkout>"
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

Use `fud2 --help` for an overview of the command-line interface.
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

There are also some subcommands for doing things other than building stuff:

* `fud2 edit-config`: Open the fud2 configuration file in `$EDITOR`.
* `fud2 list`: Print out all the available states and operations.
* `fud2 get-rsrc FILE`: Fetch a *resource file* and place it in the working directory. You typically do not need to use this interactively; it is used during builds to obtain files included with fud2 that are necessary for a given build.

[graphviz]: https://graphviz.org

## The Design of fud2

<div class="warning">

This section is about the *implementation* of fud2; it is only relevant if you want to work on it yourself.
No need to read any farther if all you want is to *use* fud2.

</div>

### fud2 is a Command Orchestrator

fud2 consists of two pieces, which are two separate Rust crates:

* FudCore (the `fud-core` crate): is a *generic compiler driver* library. This library is not specific to Calyx and could hypothetically be used to build a fud2-like driver for any compiler ecosystem. Clients of the `fud-core` library work by constructing a `Driver` object that encapsulates a set of *states* and *operations* that define the driver's behavior.
* fud2 itself is a program that uses the FudCore library. All of the Calyx-specific logic lives in `fud2`. For the most part, all of the code in the `fud2` crate consists of declaring a bunch of states and operations. The `main` function does little more than dispatch to the resulting `Driver` object's generic command-line interface.

The central design philosophy of FudCore (and by extension, fud2 itself) is that its sole job is to orchestrate external functionality.
All that functionality must be available as separate tools that can be invoked via the command line.
This is an important goal because it means the driver has a clear, discrete goal: *all it does* is decide on a list of commands to execute to perform a build.
All the "interesting work" must be delegated to separate tools outside of the driver.
This philosophy has both advantages and disadvantages:

* On the positive side, it forces all the interesting logic to be invokable via a command that you, the user, can run equally well yourself. So if something is going wrong, there is *always* a command line you can copy and paste into your terminal to reproduce the problem at that particular step. It also means that the input and output of every step must be written to files in the filesystem, so you can easily inspect the intermediate state between every command. This file-based operation also means that fud2 builds are parallel and incremental by default.
* On the other hand, requiring everything to be separate commands means that fud2 has a complicated dependency story. It is not a monolith: to get meaningful work done, you currently have to install a bunch of Python components (among other things) so fud2 can invoke them. (We hope to mitigate the logistical pain this incurs over time, but we're not there yet.) Also, writing everything to a file in between each step comes at a performance cost. Someday, it may be a performance bottleneck that two steps in a build cannot simply exchange their data directly, through memory, and must serialize everything to disk first. (This has not been a problem in practice yet.)

If you want to extend fud2 to do something new, the consequence is that you first need to come up with a sequence of commands that do that thing.
If necessary, you may find that you need to create new executables to do some minor glue tasks that would otherwise be implicit.
Then "all you need to do" is teach fud2 to execute those commands.

### States, Operations, and Setups

You can think of a FudCore driver as a graph, where the vertices are *states* and the edges are *operations*.
(In fact, this is literally the graph you can visualize with `-m dot`.)
Any build is a transformation from one state to another, traversing a path through this graph.
The operations (edges) along this path are the commands that must be executed to transform a file from the initial state to the final state.

To make fud2 do something new, you probably want to add one or more operations, and you may need to add new states.
Aside from declaring the source and destination states,
operations generate chunks of [Ninja][] code.
So to implement an operation, you write a Rust function with this signature:

    fn build(emitter: &mut Emitter, input: &str, output: &str)

Here, `emitter` is a wrapper around an output stream with a bunch of utility functions for printing out lines of Ninja code.
`input` and `output` are filenames.
So your job in this function is to print (at least) a Ninja `build` command that produces `output` as a target and uses `input` as a dependency.
For example, the Calyx-to-Verilog compiler operation might emit this chunk of Ninja code:

    build bar.sv: calyx foo.futil
      backend = verilog

when the `input` argument above is `"foo.futil"` and the `output` is `"bar.sv"`.
(The FudCore library will conjure these filenames for you; your job in this operation is just to use them as is.)

Notice here that the generated Ninja chunk is using a build rule called `calyx`.
This also needs to be defined.
To set up things like variables and build rules that operations can use, FudCore has a separate concept called *setups*.
A setup is a function that generates some Ninja code that might be shared among multiple operations (or multiple instances of the same operation).
For example, our setup for Calyx compilation generates code like this:

    calyx-base = /path/to/calyx
    calyx-exe = $calyx-base/target/debug/calyx
    rule calyx
      command = $calyx-exe -l $calyx-base -b $backend $args $in > $out

That is, it defines two Ninja variables and one Ninja rule—the one that our build command above uses.
We *could* have designed FudCore without a separation between setups and operations, so this rule would get declared right next to the `build` command above.
But that would end up duplicating a lot of setup code that really only needs to appear once.
So that's why setups exist: to share a single stanza of Ninja scaffolding code between multiple operations.
