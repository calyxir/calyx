# Profiling Scripts

This directory contains scripts for a first pass at profiling cycle counts in Calyx programs. It contains:

- `get-profile-counts-info.sh`: A wrapper script that produces a cycle counts estimate and flame graphs given a Calyx program
- `parse-vcd.py`: A helper script that reads in a VCD file and a JSON FSM file to generate cycle count estimates
- `create-visuals.py`: A helper script that reads in a cycle count report JSON (produced by `parse-vcd.py`) and produces `.folded` files for flame graphs, and JSON files for timelines

### Usage

- To run the profiling pipeline, you can run `get-profile-counts-info.sh` providing the Calyx file and the Memory data. ex) From the Calyx root directory
```
bash tools/vcd-parsing/get-profile-counts-info.sh examples/tutorial/language-tutorial-compute.futil examples/tutorial/data.json
```
