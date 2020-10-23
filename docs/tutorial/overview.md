# Building a Frontend for FuTIL

In this tutorial, we're going to build a compiler for a small language called MrXL, which consists of `map` and `reduce` operations on arrays. Here's an example of a dot product implementation:

```
input avec: int[1024]
input bvec: int[1024]
output dot: int
prodvec := map 16 (a <- avec, b <- bvec) { a * b }
dot := reduce 4 (a, b <- prodvec) 0 { a + b }
```

We're going to first write a naive compiler with some simplifying assumptions:
- Every array in a MrXL program has the same length.
- Every integer in our generated hardware will be 32 bits.
- Our generated hardware will not have any parallelism.

Once we've got this simplified compiler working, we can improve it to not require these assumptions any more.

## Code Generation

The skeleton of a FuTIL program has three sections, and looks like this:
```
component main() -> {
  cells { }
  wires { }
  control { }
}
```

Once we parse our MrXL program into an [AST][astcode], we can decide on rules for generating code depending on which AST node we're working on. Depending on the AST node, we might need to add code to `cells`, `wires` or `control`.

### `Decl` nodes

`Decl` nodes instantiate new memories and registers. Here's FuTIL code that creates a new memory `foo`:
```
foo = prim std_mem_d1(32, 4, 32);
```

For each `Decl` node, we just determine if we're instantiating a memory or a register, and then translate that to a corresponding FuTIL declaration and place that inside the `cells` section of our generated program.

### `Map` and `Reduce` nodes

For every map or reduce node, we need to generate FuTIL code that iterates over an array, performs some kind of computation, and then stores the result of that computation in another array if we're doing a map, or in an accumulator register if we're doing a reduce. We'll need to generate a few FuTIL groups:
- `incr_idx`: Increments an `idx` register using an adder. This group is done when the `idx` register is written to.
- `cond`: Applies a "less than" operator to `idx`, and the length of our input arrays.
- `eval_body`: Reads from an array, performs some kind of computation, and writes the result of the computation to an accumulator register or another array.

We'll make these groups for each `Map` and `Reduce` node, so to avoid naming collisions, we'll suffix each group with an integer starting at 0, incrementing each time we need to add new sets of  groups. These groups will be added to the `wires` section. We'll also need to add logic to the `control` section as well that uses these groups to process arrays:
```
while le0.out with cond0 {
  seq { eval_body0; incr_idx0; }
}
```





[astcode]: https://github.com/cucapra/futil/blob/mrxl/mrxl/mrxl/ast.py
