MrXL
====

MrXL is a frontend that performs `map` and `reduce` operations on arrays.
For example, this is a dot product implementation:

```
input avec: int[1024]
input bvec: int[1024]
output dot: int
prodvec := map 16 (a <- avec, b <- bvec) { a * b }
dot := reduce 4 (a, b <- prodvec) 0 { a + b }
```

Here, the numbers that come right after `map` and `reduce` (16 and 4 respectively) are "parallelism factors" that guide the generation of hardware.

Further documentation, featuring instructions for installation, interpretation, and compilation to Calyx, can be found [here](https://docs.calyxir.org/frontends/mrxl.html).
