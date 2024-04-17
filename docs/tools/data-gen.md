# Data Gen

Data Gen is a tool that can automatically generate a memory .json file from a Calyx file.
It reads the Calyx file and generates an entry in the json for each cell marked with the
`@external` attribute.

Currently, there are two parameters you can specify: 1) whether the data should be fixed point (integer is default) and 2) whether data should be randomly generated (0 is the default).
This tool can only generate fixed point values that have 16 bits for the fraction.

## How to Run
The following command can be run to generate unsigned integer zeroes:
`cargo run -p data_gen -- <calyx file>`

To generate random fixed point numbers, run:
`cargo run -p data_gen -- <calyx file> -f true -r true`

It will print the json in the command line.

## Current Limitations
As you can see, the tool is right now pretty limited.
For example, the fixed point values must be 16 bit, and the json generated only supports one memory type (for example, if you wanted some of the memories to be fixed point and others to be integers, this tool does not support that).
Ideally, we would want each Calyx memory cell to have its own attribute(s) which can hold information about what type of number representation format the memory wants. This [github issue](https://github.com/calyxir/calyx/issues/1163) goes into more detail about future improvements for the tool.