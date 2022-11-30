# Building a Frontend for Calyx

In this tutorial, we're going to build a compiler for a small language called MrXL.

## MrXL Overview
MrXL provides constructs to create arrays, and perform `map` and `reduce` operations on those arrays. Here's an example of a dot product implementation in MrXL:

```
{{#include ../../frontends/mrxl/test/dot.mrxl}}
```

We define the interface of program by specifying `input` and `output` arrays.
Input arrays have their values populated by an external harness while the output arrays must be computed using the program.

A `map` expression iterates over multiple arrays of the same element and produces a new vector using the function provided in the body.
In the above example, the `map` expression multiplies the values of `avec` and `bvec`.
`map 1` states that the operation has a *parallelism factor* of which means that the loop iterations are performed sequentially.

`reduce` expressions walk over memories and accumulate a result into a register.
In the above code snippet, we add together all the elements of `prodvec` and place them in a register named `dot`.
Since the `reduce` parallelism factor is also 1, the reduction is performed sequentially.


## Run a MrXL Program

Once we have [installed the mrxl command line tool][mrxldocs], we can run MrXL programs using [`fud`][fud].

To provide MrXL program with input values, we use fud's [JSON][json]-based [data format][fud-data].
Let's try to run this program, which has a parallelism factor of two:

```
{{#include ../../frontends/mrxl/test/add.mrxl}}
```
In order to take advantage of the parallelism in the program, the MrXL compiler automatically partitions the input memory `foo` into two different *physical banks*: `foo_b0` and `foo_b1`.
Therefore, we split up our logical `foo` input of `[1, 2, 3, 4]` into `[1,2]` and `[3,4]`:
```json
{{#include ../../frontends/mrxl/test/add.mrxl.data:2:23}}
```
Our complete data file similarly splits up the input for `baz`.

Run the program with the complete data by typing:

```
fud exec frontends/mrxl/test/add.mrxl \
    --from mrxl \
    --to vcd -s verilog.data frontends/mrxl/test/add.mrxl.data
```

## Compiling MrXL to Calyx

This guide will walk you through the steps to build a Python program that compiles MrXL programs to Calyx code.
The guide assumes some basic familiarity with Calyx.
Take a look at [Calyx tutorial][calyx-tut] if you need a refresher.

To simplify things, we'll make a few assumptions about MrXL programs:
- Every array in a MrXL program has the same length.
- Every integer in our generated hardware will be 32 bits.
- Every `map` and `reduce` body will be either a multiplication or addition of either an array element or an integer.

The following sections will outline these two high level tasks:
1. Parse MrXL into a representation we can process with Python
1. Generate Calyx code

### Parse MrXL into an AST

To start, we'll parse this MrXL program into a Python AST representation. We chose to represent [AST][astcode] nodes with Python `dataclass`.
A program is a sequence of array declarations followed by computation statements:
```python
{{#include ../../frontends/mrxl/mrxl/ast.py:prog}}
```

`Decl` nodes correspond to array declarations like `input avec: int[1024]`, and carry data about whether they're an `input` or `output` array, their name, and their type:

```python
{{#include ../../frontends/mrxl/mrxl/ast.py:decl}}
```

`Stmt` nodes represent statements such as `dot := reduce 4 (a, b <- prodvec) 0 { a + b }`, and contain more nested nodes representing their function header and body, and type of operation.

```python
{{#include ../../frontends/mrxl/mrxl/ast.py:stmt}}
```

[The complete AST][mrxl-ast] defines the remaining AST nodes required to represent a MrXL program.

### Generate Calyx Code

The skeleton of a Calyx program has three sections, and looks like this:

```
component main() -> {
  cells {}
  wires {}
  control {}
}
```

The [cells section][lf-cells] instantiates hardware units like adders, memories and registers.
The [wires section][lf-wires] contains [groups][lf-groups] that connect
together hardware instances to perform some logical task such as incrementing a specific register.
Finally, the [control section][lf-control] *schedules* the execution of groups using control operators such as `seq`, `par`, and `while`.

We perform syntax-directed compilation by walking over nodes in the above AST and generating `cells`, `wires`, and `control` operations.


#### `Decl` nodes

`Decl` nodes instantiate new memories and registers. We need these to be instantiated in the `cells` section of our Calyx output. Here's Calyx code that creates a new memory `foo`, with 4 32-bit elements and a 32-bit indexor:

```
foo = std_mem_d1(32, 4, 32);
```

For each `Decl` node, we need to determine if we're instantiating a memory or a register, and then translate that to a corresponding Calyx declaration and place that inside the `cells` section of our generated program. Here's some code from our compiler that walks through each register and memory declaration, and generates a Calyx program with those registers:

{{#include ../../frontends/mrxl/mrxl/gen_futil.py:283:290}}

(`emit_mem_decl` emits a string of the form `"mem_name = std_mem_d1(<element_width>, <num_elements>, <index_width>)"`.)

#### `Map` and `Reduce` nodes

For every map or reduce node, we need to generate Calyx code that iterates over an array, performs some kind of computation, and then stores the result of that computation. For `map` operations, we'll perform a computation on an element of an input array, and then store the result in a result array. For `reduce` operations, we'll also use an element of an input array, but we'll also use an _accumulator_ register that we'll use in each computation, and we'll also store to. For example, if we were writing a `reduce` that summed up the elements of an input array, we'd use an accumulator register that was initialized to hold the value 0, and add to the value of this register each element of an input array.

We can implement these behaviors using Calyx groups:
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

### Add Parallelization

MrXL allows you to parallelize your `map` and `reduce` operations. Let's revisit the `map` example from earlier:

```
input foo: int[4]
output baz: int[4]
baz := map 4 (a <- foo) { a + 5 }
```

The number 4 after the `map` specifies the number of adders we can use at once to parallelize this computation. There are a few ways we could parallelize this program, and one of them is to split the memories used in the `map` operation into 4 separate memory _banks_, and then we can read from each bank of `foo` and write into each bank of `baz` simultaneously. In general, we can break memories of size `m` into `b` banks (each with size `m/b`), and then simultaneously process those `b` banks. Realizing this in Calyx means creating separate memories for each bank, and creating `group`s to process each bank. Here's a section of the compiler that generates banked memories:

```
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:4:18}}
```

In the `Map` and `Reduce` code generation section we described `group`s that could be orchestrated to iterate over a memory and process it. We'll now have to do that for each memory bank, and then parallelize these operations in the generated Calyx's `control` section. We can accomplish this with Calyx's `par` keyword, signalling to execute groups in parallel. Here's an example of executing four while loops in parallel:

```
par {
  while le_b0.out with cond_b0 { seq { eval_body_b0; incr_idx_b0; } }
  while le_b1.out with cond_b1 { seq { eval_body_b1; incr_idx_b1; } }
  while le_b2.out with cond_b2 { seq { eval_body_b2; incr_idx_b2; } }
  while le_b3.out with cond_b3 { seq { eval_body_b3; incr_idx_b3; } }
}
```

## Conclusion

Hopefully this should be enough to get you started with writing your own MrXL compiler. Some more follow up tasks you could try if you're interested:
- Implement code generation to implement `reduce` statements, which we do not include in our compiler.
- Implement code generation that allows memories that differ from one another in size.
- Implement complex function body expressions. We only support binary operations with simple operands, like `a + 5`. Different hardware components take multiple cycles to execute: for example, a register takes 1 cycle to write data to, but a memory might take more. This complicates hardware design, as you need to account for differing latencies among hardware components.

[astcode]: https://github.com/cucapra/futil/blob/mrxl/mrxl/mrxl/ast.py
[mrxldocs]: https://github.com/cucapra/futil/tree/master/frontends/mrxl
[fud]: ../fud/index.md
[fud-data]: ../lang/data-format.md
[json]: https://www.json.org/json-en.html
[calyx-tut]: ./language-tut.md
[mrxl-ast]: https://github.com/cucapra/calyx/blob/master/frontends/mrxl/mrxl/ast.py
[lf-cells]: ../lang/ref.md#cells
[lf-wires]: ../lang/ref.md#the-wires-section
[lf-groups]: ../lang/ref.md#group-definitions
[lf-control]: ../lang/ref.md#the-control-operators
