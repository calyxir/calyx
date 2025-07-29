# synthrep

**Vivado report analysis and visualization tool**

`synthrep` is a command-line utility for parsing and visualizing Vivado `.rpt` files. It helps extract area, utilization and timing summaries and generate visual breakdowns of resource usage.

## Install

```bash
uv tool install .
```

## Usage

```bash
synthrep <command> [options]
```

Use `-h` or `--help` after any command to see available options:

```bash
synthrep -h
synthrep viz -h
```

### Commands

#### `summary`

Extracts a JSON summary of synthesis and implementation resource usage, as well as timing info.

```bash
synthrep summary [-d DIRECTORY] [-m {utilization,hierarchy}]
```

**Options:**

- `-d`, `--directory` – specify Vivado output directory (default: `out`)
- `-m`, `--mode` – set summary mode (default: `utilization`)

There are two modes:

- `utilization`: prints a flat summary of total resource usage and timing results:
  - LUTs, FFs, DSPs, BRAMs
  - Carry chains, muxes
  - Timing met/not met
  - Worst slack and clock frequency

- `hierarchy`: prints the full utilization hierarchy, which can be passed to the profiler to obtain cycle-resource-utilization data and visualizations

**Example:**

```bash
synthrep summary -m hierarchy > hierarchy.json
```

#### `viz`

Visualize area usage using different plot types. Defaults to a treemap.

```bash
synthrep viz [--type TYPE] [--filename FILE] [--column COL] [-v]
```

**Options:**

- `-t`, `--type` – one of `treemap`, `sunburst`, `icicle`, `flamegraph`
- `-f`, `--filename` – path to a `.rpt` file (default: `out/hierarchical_utilization_placed.rpt`)
- `-c`, `--column` – one of `ff`, `lut`, `llut`, `lutram` (default: `ff`)
- `-v`, `--verbose` – prints the parsed data table

**Example:**

```bash
synthrep viz -t sunburst -c lut
```

### Output

- Visuals (via Plotly on the default browser)
- Folded stack format for flamegraphs (stdout)

A clone of [Brendan Gregg's FlameGraph repository](https://github.com/brendangregg/FlameGraph) is needed to generate FlameGraph SVGs. The folded stack output can be directly chained into the FlameGraph Perl script:

```bash
synthrep viz -c lut -t flamegraph | path/to/FlameGraph/flamegraph.pl > flamegraph.svg  
```

## Supported formats

- Vivado `.rpt` files from:
  - synthesis (e.g. `main_utilization_synth.rpt`)
  - implementation (e.g. `main_utilization_placed.rpt`)
  - timing (e.g. `main_timing_summary_routed.rpt`)
