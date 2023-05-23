# Examples
The easiest way to play with these examples is using the Calyx driver
[fud](https://docs.calyxir.org/tools/fud.html). Provided in the documentation
is a thorough guide to how to use the driver.

# Extensions
In each directory, you'll notice there are a few different types of extensions:
 - `.fuse` is the [Dahlia](https://capra.cs.cornell.edu/fuse/docs/overview/) extension.
 - `.futil` is the [Calyx](https://docs.calyxir.org/intro.html) extension, also referred to as Calyx.
 - `.data` is an extension alias for `.json`, which is how we pre-load data into simulated memories.
 - `.expect` is used for [runt](https://docs.calyxir.org/tools/runt.html) to ensure
 our examples remain up-to-date.

 You might notice that some files share common names, but different extensions, e.g.
 `examples/dahlia/dot-product.fuse` and `examples/futil/dot-product.futil`. Here, the `dot-product`
 implementation was simply lowered from Dahlia to Calyx,
 so one can simulate with `examples/dahlia/dot-product.fuse.data` for either.

# Commands
Listed below are just a few commands to get yourself going.

### Dahlia
To simulate an example in `examples/dahlia`:
```
fud e examples/dahlia/dot-product.fuse --to dat -s \
verilog.data examples/dahlia/dot-product.fuse.data
```

To lower Dahlia to Calyx:
```
fud e examples/dahlia/dot-product.fuse --to calyx
```

### Calyx
To simulate an example in `examples/futil`:
```
fud e examples/futil/dot-product.futil --to dat -s \
verilog.data examples/dahlia/dot-product.fuse.data
```

To lower Calyx to Verilog:
```
fud e examples/futil/simple.futil --to verilog
```
