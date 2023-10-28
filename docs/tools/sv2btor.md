# `sv2btor` 

The `sv2btor` tool is a tool that leverages `yosys` and
`verible` to translate the SystemVerilog files into BTOR2. 

# Usage

```bash
Usage: sv2btor.py <PATH_TO_YOSYS_EXECUTABLE>  <PATH_TO_VERIBLE_VERILOG_SYNTAX>  <OUTPUT_DIR>  <VERILOG_FILE [VERILOG_FILE [...]]>
```

# Installation
To run this tool, you need to have `yosys` and `verible-verilog-syntax` installed. You will also need the `anytree` python package.

- You can install `yosys` by following the instructions [here](https://github.com/YosysHQ/yosys).
- You can install `verible-verilog-syntax` by following the instructions [here](https://github.com/chipsalliance/verible). Note that we only need the standalone `verible-verilog-syntax` executable, the rest of the tools are optional.
- You can install `anytree` by running `pip install anytree`.

