# Examples
The easiest way to play with these examples is using the FuTIL driver 
[fud](https://capra.cs.cornell.edu/calyx/tools/fud.html). Provided in the documentation
is a thorough guide to how to use the driver. 

# Extensions
In each directory, you'll notice there are a few different types of extensions:
 - `.fuse` is the [Dahlia](https://capra.cs.cornell.edu/fuse/docs/overview/) extension.
 - `.futil` is the [FuTIL](https://capra.cs.cornell.edu/calyx/intro.html) extension, also referred to as Calyx.
 - `.data` is an extension alias for `.json`, which is how we pre-load data into simulated memories.
 - `.expect` is used for [runt](https://capra.cs.cornell.edu/calyx/tools/runt.html) to ensure 
 our examples remain up-to-date.
 
 You might notice that some files share common names, but different extensions, e.g.
 `examples/dahlia/dot-product.fuse` and `examples/futil/dot-product.futil`. Here, the `dot-product`
 implementation was simply lowered from Dahlia to FuTIL, 
 so one can simulate with `examples/dahlia/dot-product.fuse.data` for either. 

# Commands
Listed below are just a few commands to get yourself going.

### Dahlia 
To simulate an example in `examples/dahlia`:
```
fud e examples/dahlia/dot-product.fuse --to dat -s \
verilog.data examples/dahlia/dot-product.fuse.data
```

To lower Dahlia to FuTIL:
```
fud e examples/dahlia/dot-product.fuse --to futil
```

### FuTIL
To simulate an example in `examples/futil`:
```
fud e examples/futil/dot-product.futil --to dat \
-s examples/dahlia/verilog.data dot-product.fuse.data
```

To lower FuTIL to Verilog:
```
fud e examples/futil/simple.futil --to verilog
```