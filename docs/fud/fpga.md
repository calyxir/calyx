# FPGA Execution
TODO

## System requirements
TODO

## Calyx (futil) file setup
To execute a Calyx program on an FPGA, add a `toplevel` annotation to the main component.
```
component main<"toplevel"=1>() -> ()
```

## Command
TODO
```
fud e <futil_file> --to xclbin -v -o kernel.xclbin
```
