# Fuzz: Compiler & Interpreter Testing
Automates the process of comparing either two input files, or two backend tools. 

The input file should be convertible to Calyx, and any backends should be defined in fud to simulate/execute a program.
For the compare file functionality, two input files to be compared and a data template file are mandatory, while an input for backend tool and number of iteration are optional (icarus-verilog is the default backend tool).
For the compare backend functionality, an input file as reference, a data template, and two backend tools mandatory, but the number of iteration is optional.

(to be changed:) The current documentation for fuzz lives [here](https://docs.calyxir.org/fud/index.html).

## Contributing

You can install these tools with:
```
pip3 install fud
pip3 install argparse
pip3 install deepdiff
```
