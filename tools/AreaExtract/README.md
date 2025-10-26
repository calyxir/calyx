# AreaExtract

AreaExtract is a tool that replaces previous technology-specific frontends for
the Calyx profiler, Petal. It offers a combined frontend for several sources of
area data for accelerator designs, and outputs processed data in a common data
format that is parseable by Petal. Currently, the following technologies are
supported:

- Vivado, as hierarchical area synthesis reports
- Yosys, as both IL and statistics files

## Usage

```
$ aext -h 
usage: aext [-h] [-o OUTPUT] {vivado,yosys} ...

Parse FPGA synthesis reports into a Common Data Format.

Supported origins:
  - Vivado: single hierarchical .rpt file
  - Yosys: .il (intermediate language) and .json (stat) file

Output is a JSON serialization of the Common Data Format.

positional arguments:
  {vivado,yosys}
    vivado              parse a Vivado utilization .rpt file
    yosys               parse Yosys IL and stat JSON files

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
                        optional output file for JSON (defaults to stdout)
```

## Obtaining area data

This section provides instructions to obtain area data for designs from
supported technologies, to use as input for AreaExtract.

### Vivado

The simplest way to obtain a hierarchical area RPT file is to use Fud2 to run
synthesis on a Calyx design:

```
fud2 <design>.futil --to area-report > <report>.rpt
```

Alternatively, it is possible to use Fud2 to obtain a synthesis-ready Verilog
file, and then use Vivado directly to conduct synthesis. The relevant TCL
command for Vivado is:

```
report_utilization -hierarchical -file <report>.rpt
```

### Yosys

Using the OSS-CAD suite, IL and statistics files can be obtained as follows:

```
yosys -p "read_verilog -sv <VERILOG_FILE>.sv; hierarchy -top main; opt; write_rtlil <IL_FILE>.il; tee -o <STAT_FILE>.json stat -json"
```

It is also possible to pass Liberty files to [the `stat` command](https://yosyshq.readthedocs.io/projects/yosys/en/0.47/cmd/stat.html)
through the flag `-liberty <file>`.

## Future work

This tool is not yet a full replacement of its technology-specific predecessors,
`synthrep` for Vivado and `aprof` for Yosys, as it is not able to produce area-only
visualizations, which is a desirable feature. In addition, some of `synthrep`'s
functionality is unrelated to area, and is not in scope for AreaExtract. Another
area that is being explored is the addition of other technologies, especially
OpenROAD as it targets ASICs instead of FPGAs. While Yosys also offers ASIC
capabilities, it is primarily oriented towards FPGAs; Vivado exclusively targets
AMD FPGAs.
