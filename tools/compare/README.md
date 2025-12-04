# Compare

Compare is a tool for comparing simulation and synthesis results for multiple
Calyx designs. It reads a CSV recipe, runs the corresponding
simulation and synthesis flows, and produces a summary table (CSV or visual).

## Input

| Column        | Description                                                         |
|---------------|---------------------------------------------------------------------|
| `DESIGN`      | Calyx design file (e.g., `example.futil`)                           |
| `COMP_SIM`    | `"True"` to run simulation                                          |
| `SIM_DATA`    | Input data file for simulation                                      |
| `COMP_SYNTH`  | `"True"` to run synthesis and place & route                         |
| `SYNTH_VAR`   | Area variable to extract (e.g., `ff`)                               |
| `SYNTH_PERIOD`| Clock period used for synthesis (e.g., `7.00`)                      |

## Output

| Column        | Description                                   |
|---------------|-----------------------------------------------|
| `DESIGN`      | Design name                                   |
| `SIM_CYCLES`  | Cycle count from simulation                   |
| `SYNTH_STATUS`| Whether timing was met                        |
| `SYNTH_AREA`  | Extracted area value                          |
| `EXEC_TIME`   | `SIM_CYCLES x SYNTH_PERIOD`                   |

## Installation and usage

```bash
$ uv tool install .
$ compare -h                                                                                                                                 (base) 
usage: compare [-h] [-v] [-p] [-o OUTPUT] input
```

The default output is `stdout`, but an output file can be specified with `-o`.
The table can also be printed in "visual mode" with `-p`. Verbose mode can be
enabled with `-v`.
