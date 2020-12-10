# Building a Frontend for FuTIL

In this tutorial, we're going to build a compiler for a small language called MrXL, which consists of `map` and `reduce` operations on arrays. Here's an example of a dot product implementation:

```
input avec: int[1024]
input bvec: int[1024]
output dot: int
prodvec := map 16 (a <- avec, b <- bvec) { a * b }
dot := reduce 4 (a, b <- prodvec) 0 { a + b }
```

A `map` expressions produces a new vector, each element an evaluated expression that can use elements of other vectors. In the above example, the `map` expression multiplies the values of `avec` and `bvec`. These expressions also have _parallelism factors_: in the above code snippet, the `map` expression has a parallelism factor of 16, which means we stamp out 16 multipliers to speed up the computation.

`reduce` expressions walk over memories and accumulate a result into a register. In the above code snippet, we add together all of the elements of `prodvec` and place them in a register named `dot`.

Here's how you compile a MrXL program to FuTIL and then Verilog, and run that Verilog code with Verilator:

```
fud exec frontends/mrxl/test/add.mrxl --from mrxl --to vcd_json
```

This guide will walk you through the steps to build a program that compiles MrXL programs to FuTIL code. To simplify things, we'll make a few assumptions:
- Every array in a MrXL program has the same length.
- Every integer in our generated hardware will be 32 bits.
- Every `map` and `reduce` body will be either a multiplication or addition of either an array element or an integer.

## Code Generation

The skeleton of a FuTIL program has three sections, and looks like this:

```
component main() -> {
  cells { }
  wires { }
  control { }
}
```

`cells` contains declarations for logical hardware units like adders, memories and registers. `wires` contains `group`s that connect together the units declared in `cell`s and form the structure of the hardware. `control` contains the logic specifying when the `group`s will perform their computation.

First, we'll parse our program into an [AST][astcode]. This program:

```
input foo: int[4]
output baz: int[4]
baz := map 4 (a <- foo) { a + 5 }
```

has an AST that we could represent in Python like this:

```
Prog(
  decls=[
    Decl(input=True, name='foo', type=Type(base='Tree(int, [])', size=4)),
    Decl(input=False, name='baz', type=Type(base='Tree(int, [])', size=4))
  ],
  stmts=[
    Stmt(dest='baz',
      op=Map(
        par=4,
        bind=[Bind(dest=['a'], src='foo')],
        body=BinExpr(
          op='add',
          lhs=VarExpr(name='a'),
          rhs=LitExpr(value=5)
        )
      )
    )
  ]
)
```

Now we can decide on rules for generating code depending on which AST node we're working on. Depending on the AST node, we might need to add code to `cells`, `wires` or `control`.

### `Decl` nodes

`Decl` nodes instantiate new memories and registers. We need these to be instantiated in the `cells` section of our FuTIL output. Here's FuTIL code that creates a new memory `foo`, with 4 32-bit elements and a 32-bit indexor:

```
foo = prim std_mem_d1(32, 4, 32);
```

For each `Decl` node, we need to determine if we're instantiating a memory or a register, and then translate that to a corresponding FuTIL declaration and place that inside the `cells` section of our generated program. To get you started, here's some code that walks through each register declaration and generates a FuTIL program with those registers:

```
def emit_reg_decl(name, size):
    return "{} = prim std_reg({});".format(name, 32)

# Collect memory and register declarations.
cells = []
for decl in prog.decls:
    if decl.type.size:  # A memory
        ...
    else:  # A register
        cells.append(emit_reg_decl(decl.name, 32))

futil_program = '''
component main() -> {{
  cells {{ {} }}
  wires {{ }}
  control {{ }}
}}
'''.format("\n".join(cells))
```

### `Map` and `Reduce` nodes

For every map or reduce node, we need to generate FuTIL code that iterates over an array, performs some kind of computation, and then stores the result of that computation. For `map` operations, we'll perform a computaiton on an element of an input array, and then store the result in a result array. For `reduce` operations, we'll also use an element of an input array, but we'll also use an _accumulator_ register that we'll use in each computation, and we'll also store to. For example, if we were writing a `reduce` that summed up the elements of an input array, we'd use an accumulator register that was initialized to hold the value 0, and add to the value of this register each element of an input array.

We can implement these behaviors using FuTIL groups:
- `incr_idx`: Increments an `idx` register using an adder. This group is done when the `idx` register is written to.
- `cond`: Applies a "less than" operator to `idx`, and the length of our input arrays, using the `le` hardware unit.
- `eval_body`: Reads from an array, performs some kind of computation, and writes the result of the computation to an accumulator register or another array.

We'll make these groups for each `Map` and `Reduce` node, so to avoid naming collisions, we'll suffix each group with an integer starting at 0, incrementing each time we need to add a new set of  groups. These groups will be added to the `wires` section. We'll also need to add logic to the `control` section as well that uses these groups to process arrays:

```
while le0.out with cond0 {
  seq { eval_body0; incr_idx0; }
}
```

This logic orchestrates our groups, basically representing iterating over our array and evaluating some computation on each element of the array. On each iteration we signal for the `eval_body0` group to do its work, followed sequentially by `incr_idx0` to advance our index register so that we can work on the next element of the array.

### Parallelization

MrXL allows you to parallelize your `map` and `reduce` operations. Let's revisit the `map` example from earlier:

```
input foo: int[4]
output baz: int[4]
baz := map 4 (a <- foo) { a + 5 }
```

The number 4 after the `map` specifies the number of adders we can use at once to parallelize this computation. There are a few ways we could parallelize this program, and one of them is to split the memories used in the `map` operation into 4 separate memory _banks_, and then we can read from each bank of `foo` and write into each bank of `baz` simultaneously. In general, we can break memories of size `m` into `b` banks (each with size `m/b`), and then simultaneously process those `b` banks. Realizing this in FuTIL means creating separate memories for each bank, and creating `group`s to process each bank. In the `Map` and `Reduce` code generation section we described `group`s that could be orchestrated to iterate over a memory and process it. We'll now have to do that for each memory bank, and then parallelize these operations in the generated FuTIL's `control` section. We can accomplish this with FuTIL's `par` keyword, signalling to execute groups in parallel. Here's an example of executing four while loops in parallel:

```
par {
  while le_b0.out with cond_b0 { seq { eval_body_b0; incr_idx_b0; } }
  while le_b1.out with cond_b1 { seq { eval_body_b1; incr_idx_b1; } }
  while le_b2.out with cond_b2 { seq { eval_body_b2; incr_idx_b2; } }
  while le_b3.out with cond_b3 { seq { eval_body_b3; incr_idx_b3; } }
}
```

### Scheduling

Currently, our compiler only supports multipication and addition operations between two values that are either integers or read from an array. To have more complex expressions, even something like `a + b + 5`, involves creation of multiple `group`s that will need to have their execution explicitly scheduled. We are currently in the process of deciding where scheduling should be handled: at the FuTIL level, or at the frontend level. Stay tuned!


[astcode]: https://github.com/cucapra/futil/blob/mrxl/mrxl/mrxl/ast.py
