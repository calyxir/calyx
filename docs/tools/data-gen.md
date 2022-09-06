# Data Gen 

Data Gen is a tool that can generate a memory json from a Calyx file. It reads 
the Calyx file and generates an entry in the json for each cell marked with the 
`@external` attribute. Currently, there are two types of number representation formats that can be generated as the data for the memory json: 1) unsigned 32 bit bitnums all equal to 0 and 2) signed, 32 bit fixed point numbers with `frac_width` = 16, and the data is randomly generated.   

## How to Run 
The following command can be run to generate unsigned, 32-bit zeroes:   
`cargo run -p data_gen -- <calyx file>`  

To generate random fixed point numbers, run:   
`cargo run -p data_gen -- <calyx file> -f true`   

It will print the json in the command line  

## Current Limitations 
As you can see, the tool is right now pretty limited, because it only supports 2 different representations of numbers. What if you want to generate random, 8 bit, unsigned ints in your memory? Data Gen currently wouldn't support that. Ideally, we would want each Calyx memory cell to have its own attribute(s) which can hold information about what type of number representation format the memory wants. This [github issue](https://github.com/cucapra/calyx/issues/1163) goes into more detail about future improvements for the tool. 