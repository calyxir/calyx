# The Calyx Profiler

*Note: The profiler is still in development. If you have any suggestions, thoughts, or feedback, please let us know!*

Profilers can help you analyze performance information to find places you can optimize in your code. Currently, the Calyx profiler prouduces timing information in terms of cycle counts.

## Setup

In order to run the profiler, you need:
- [fud2](./fud2/index.html)
- Internal tools within the Calyx repository; run `cargo build --all` to build all of them.
- The Python [`vcdvcd` library](https://github.com/cirosantilli/vcdvcd). Running `fud2 env init` should install this for you.
- A clone of Brendan Gregg's Flamegraph repository: [https://github.com/brendangregg/FlameGraph](https://github.com/brendangregg/FlameGraph)

Then, you need to edit your fud2 configuration file to specify the location of `flamegraph.pl` within the `Flamegraph` repository:

ex)
```
[flamegraph]
script = "/home/ayaka/projects/FlameGraph/flamegraph.pl"
```

## Basic Use

To obtain a flame graph, run fud2 with an SVG output file:

ex)
```
fud2 tests/correctness/pow.futil -o pow.svg -s sim.data=tests/correctness/pow.futil.data
```

The produced flame graph is "flattened", which means that each parallel arm gets its own "cycle". So, if arm A and arm B were executing on a single cycle, the flame graph would account for a cycle in arm A and a cycle in arm B. You can view and interact with the flame graph using your favorite web browser.

If you retain the fud2 build directory with the `--keep` option or the `--dir` option, you will find additional profiler output files in `<FUD2_BUILD_DIRECTORY>/profiler-out`:

  - `scaled-flame.svg`: A scaled flame graph, where a cycle is divided between the parallel arms in execution. So, if arm A and arm B were executing on a single cycle, the flame graph would account for 0.5 cycles in arm A and 0.5 cycles in arm B.
  - `aggregate.dot.png`: A tree summary of the execution of the program. Nodes (groups and cells) are labeled with the number of times the node was a leaf, and edges are labeled with the number of cycles that edge was activated.
  - `rank{i}.dot.png`: A tree representation of the `i`th most active stack picture. `rankings.csv` lists the specific cycles that each ranked tree was active for.
