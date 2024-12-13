# Profiling Scripts

This directory contains scripts for a first pass at profiling cycle counts in Calyx programs. It contains:

- `get-profile-counts-info.sh`: A wrapper script that produces a cycle counts estimate and flame graphs given a Calyx program
- `parse-vcd.py`: A helper script that reads in a VCD file and a JSON FSM file to generate cycle count estimates
- `create-visuals.py`: A helper script that reads in a cycle count report JSON (produced by `parse-vcd.py`) and produces `.folded` files for flame graphs, and JSON files for timelines

### Usage

To run the profiling pipeline, you can run `get-profile-counts-info.sh` providing the Calyx file and the Memory data. ex) From the Calyx root directory
```
bash tools/profiler/get-profile-counts-info.sh examples/tutorial/language-tutorial-compute.futil examples/tutorial/data.json
```

The script will create a directory `<CALYX>/tools/profiler/data/<CALYX_FILE_SHORTNAME>` where `<CALYX_FILE_SHORTNAME>` is the part of your `.futil` file before the `.futil` (ex. the shortname for `language-tutorial-compute.futil` would be `language-tutorial-compute`.) This directory contains two subdirectories:

- `generated-data`: Contains profiler results, flame graphs, and intermediate data
- `logs`: Contains log files from each profiling step

Some important files to check out in `generated-data`:

- `flame.svg`: The cycle counts flame graph. Can be zoomed in when viewed using a web browser
- `summary.csv`: Contains a text view of the profiled information.
- `timeline.json`: When viewed with [Perfetto UI](https://ui.perfetto.dev/), shows a timeline view (x axis is time, where 100ns is equivalent to one cycle)

Some additional flame graphs worth taking a look at:

- `components-flame.svg`: A cycle count flame graph that aggregates cycles across all groups in a component.
- `frequency-flame.svg`: [Currently under maintenance] A flame graph that displays the number of times a group was _active_.