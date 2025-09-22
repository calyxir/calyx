# Area estimation tool

This tool estimates and visualizes hardware design areas from Yosys IL and stat files. Yosys IL and stat files can be obtained from a Verilog file via:

```bash
yosys -p "read_verilog -sv inline.sv; hierarchy -top main; opt; write_rtlil inline.il; tee -o inline.json stat -json"
```

## Install

```bash
uv tool install .
```

## Usage

```bash
aprof-parse -h
aprof-plot -h
```

### Commands

**`aprof-parse`** – convert IL + stat files into JSON summary

```bash
aprof parse <IL_FILE> <STAT_FILE> [-o OUTPUT]
```

- `-o` optional output JSON (default stdout)

**`aprof-plot`** – visualize JSON summary

```bash
aprof plot <INPUT_JSON> <MODE> [-o OUTPUT]
```

- `MODE` one of `bar`, `treemap`
- `-o` optional output HTML (default depends on mode)
