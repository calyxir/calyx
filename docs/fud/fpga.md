# FPGA

## futil file setup
In order to simulate the program, explicit annotation of `toplevel` is necessary.
```
component main<"toplevel"=1>() -> ()
```

## Command
```
fud e <futil_file> --to xclbin -v -o kernel.xclbin
```
