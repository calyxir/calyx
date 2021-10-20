# FPGA Execution
TODO: Describe FPGA

TODO: Difference between simulation and FPGA execution

Currently, we rely upon the Vitis High-Level-Synthesis (HLS) compiler to be able to execute Calyx files on FPGA. To get the cycle-accurate view of the kernel (program) activity, we are using hardware emulation `hw_emu` build target to compile Calyx into RTL to be run on FPGA and get performance estimates.

Compiling down to RTL for programs to be able to run in FPGA is quite tricky. Even with the HLS tool, the steps one needs to follow to create necessary files are burdensome. With `fud`, we aim to abstract away list of different compilation commands necessary for one to run Calyx program on FPGA. 

Note: The following tutorial is adapted from the [public Vitis tutorial repository](https://github.com/Xilinx/Vitis-Tutorials/tree/2021.1/Getting_Started).

## System requirements
In order to configure the environment, we need to first setup the environment. This assumes that all the necessary packages and files are downloaded from Xilinx website. More information can be found in the tutorial link above.

To configure the Vitis environment, we need to first source the setup scripts.
```
source <Vitis_install_path>/Vitis/2020.1/settings64.sh
source /opt/xilinx/xrt/setup.sh
```

On some Ubuntu distributions, you must also export LIBRARY\_PATH.
```
export LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
``` 

## Calyx (futil) file setup
To execute a Calyx program on an FPGA, add a `toplevel` annotation to the main component.
```
component main<"toplevel"=1>() -> ()
```

## Command
To compile and execute program on FPGA, we need to create a binary file, which we will call `kernel.xclbin`. 
```
fud e <futil_file> --to xclbin -v -o kernel.xclbin
```
