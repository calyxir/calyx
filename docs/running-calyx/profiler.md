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

To obtain a flame graph from a Calyx file, run fud2 with an SVG output file and the `--through profiler` option:

ex)
```
fud2 tests/correctness/pow.futil -o pow.svg --through profiler -s sim.data=tests/correctness/pow.futil.data
```

The produced flame graph is "flattened", which means that each parallel arm gets its own "cycle". So, if arm A and arm B were executing on a single cycle, the flame graph would account for a cycle in arm A and a cycle in arm B. You can view and interact with the flame graph using your favorite web browser.

If you retain the fud2 build directory with the `--keep` option or the `--dir` option, you will find additional profiler output files in `<FUD2_BUILD_DIRECTORY>/profiler-out`:

  - `scaled-flame.svg`: A scaled flame graph, where a cycle is divided between the parallel arms in execution. So, if arm A and arm B were executing on a single cycle, the flame graph would account for 0.5 cycles in arm A and 0.5 cycles in arm B.
  - `timeline_trace.pftrace`: A file in Perfetto's native protobuf trace format. Inputting this file into [Perfetto UI](https://ui.perfetto.dev/) will visualize a timeline of your Calyx program's simulation.

### Running the profiler for ADL programs

Currently, the profiler supports programs written in Dahlia and Calyx-Py.

#### Dahlia Profiling

Dahlia programs can be profiled by using the `dahlia-profiler` op instead of the `profiler` op.

ex)
```
fud2 examples/dahlia/dot-product.fuse -o svgs/dot-product.svg --through dahlia-profiler -s sim.data=examples/dahlia/dot-product.fuse.data
```

**Note**: The `svg` file outputted is the flattened flame graph at the Calyx level, so we highly recommend retaining the fud2 build directory.

If you retain the fud2 build directory with the `--keep` option or the `--dir` option, you will find additional Dahlia profiler output files in `<FUD2_BUILD_DIRECTORY>/profiler-out` (on top of the Calyx profiler output files listed in `Basic Use`):

  - `dahlia-flat-flame.svg`: A flattened flame graph at the Dahlia level.
  - `dahlia-scaled-flame.svg`: A scaled flame graph at the Dahlia level.
  - `dahlia_timeline_trace.pftrace`: A timeline view file at the Dahlia level in Perfetto's native protobuf trace format. Inputting this file into [Perfetto UI](https://ui.perfetto.dev/) will visualize a timeline of your Dahlia program's simulation.

#### Calyx-py Profiling

Calyx-py programs can be profiled by using the `calyx-py-profiler` op. If the program has an argument, you can provide the arguments into the profiler simulation run via the `profiler.py-args` argument.

ex)
```
fud2 frontends/queues/tests/strict/strict_6flow_test.py -o svgs/strict_6flow_test.svg --through calyx-py-profiler -s sim.data=frontends/queues/tests/strict/strict_6flow_test.data -s profiler.py-args="20000 --keepgoing"
```

**Note**: The `svg` file outputted is the flattened flame graph at the Calyx level, so we highly recommend retaining the fud2 build directory.

If you retain the fud2 build directory with the `--keep` option or the `--dir` option, you will find additional Calyx-py profiler output files in `<FUD2_BUILD_DIRECTORY>/profiler-out` (on top of the Calyx profiler output files listed in `Basic Use`):

  - `mixed-flat-flame.svg`: A flattened flame graph that shows both Calyx-py names and Calyx names.
  - `mixed-scaled-flame.svg`: A scaled flame graph that shows both Calyx-py names and Calyx names.
  - `py-flat-flame.svg`: A flattened flame graph at the Calyx-py level.
  - `py-scaled-flame.svg`: A scaled flame graph at the Calyx-py level.