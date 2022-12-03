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
`map 1` states that the operation has a *parallelism factor* of 1 which means that the loop iterations are performed sequentially.

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

> You can find our [complete implementation][impl] in the Calyx repository.

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

#### Calyx Embedded DSL

To make it easy to generate the hardware, we'll use Calyx's [`builder` module][builder-ex] in Python:
```python
import calyx.builder as cb

prog = cb.Builder() # A Calyx program
main = prog.component("main") # Create a component named "main"
```

#### `Decl` nodes

`Decl` nodes instantiate new memories and registers.
We need these to be instantiated in the `cells` section of our Calyx output.
We use Calyx's `std_reg` and `std_mem_d1` primitives to represent registers and memories:

```C
import "primitives/core.futil"; // Import standard library

component main() -> () {
  cells {
    // A memory with 4 32-bit elements. Indexed using a 6-bit value.
    foo = std_mem_d1(32, 4, 6);
    // A register that contains a 32-bit value
    r = std_reg(32);
  }
  ...
}
```

For each `Decl` node, we need to determine if we're instantiating a memory or a register, and then translate that to a corresponding Calyx declaration and place that inside the `cells` section of our generated program.

If a memory is used in a parallel `map` or `reduce`, we might need to create different physical banks for it.
We define a function to walk over the AST and compute the parallelism factor for each memory:
```python
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:compute_par_factors}}
```

Using this information, we can instantiate registers and memories for our inputs and outputs:
```python
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:collect-decls}}
```
The `main.mem_d1` call is a function defined by the Calyx builder module to instantiate memories for a component.
By setting `is_external=True`, we're indicating that a memory declaration is a part of the program's input-output interface.


### Compiling `Map` Operations

For every map or reduce node, we need to generate Calyx code that iterates over an array, performs some kind of computation, and then stores the result of that computation.
For `map` operations, we'll perform a computation on an element of an input array, and then store the result in a result array.
We can use Calyx's [while loops][lf-while] to iterate over an input array, perform the map's computation, and store the final value.
At a high-level, we want to generate the following pieces of hardware:
1. A register to store the current value of the loop index.
2. A comparator to check of the loop index is less than the array size.
3. An adder to increment the value of the index.
4. Hardware needed to implement the loop body computation.

#### Loop Condition

We define a [combinational group][lf-comb-group] to perform the comparison `idx < arr_size` that uses an `lt` cell.

```python
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:cond_group}}
```


#### Index Increment

```python
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:incr_group}}
```

The loop index increment is implemented using a [group][lf-group] and an adder (`adder`).
We provide the index's previous value and the constant 1 to the adder and write the adder's output into the register.
Because we're performing a stateful update of the register, we must wait for the register to state that it's committed the write by setting the group's done condition to the register's `done` signal.

#### Body Computation

The final piece of the puzzle is the body's computation.
The corresponding group indexes into the input memories:
```python
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:map_inputs}}
```
Because the builder module is an embedded DSL, we can simply use Python's `for` loop to generate all the required assignments for indexing.

Instantiates an adder or a multiplier depending on the computation needed using the `expr_to_port` helper function:
```python
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:map_op}}
```

And writes the value from the operation into the output memory:
```py
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:map_write}}
```
This final operation is complex because we must account for whether we're using an adder or a multiplier.
Adders are *combinational*–they produce their output immediately–while multipliers are *sequential* and require multiple cycles to produce its output.

When using a mutliplier, we need to explicitly set its `go` signal to one and only write the output from the multiplier into the memory when its `done` signal is asserted.
We do this by assigning the memory's `write_en` (write enable) signal to the multiplier's done signal.
Finally, the group's computation is done when the memory write is committed.

#### Generating Control

Once we have generated the hardware needed for our computation, we can schedule its computation using [control operators][lf-control]:

```py
{{#include ../../frontends/mrxl/mrxl/gen_futil.py:map_loop}}
```

We generate a while loop that checks that the index is less than the array size.
Then, it sequentially executes the computation for the body and increments the loop index.

### Add Parallelization

MrXL allows you to parallelize your `map` and `reduce` operations. Let's revisit the `map` example from earlier:

```
input foo: int[4]
output baz: int[4]
baz := map 4 (a <- foo) { a + 5 }
```

The number 4 specifies that four copies of the loop bodies should be executed in parallel.
Our implementation already creates [memory banks](#decl-nodes) to allow for parallel accesses.
At a high-level, we can change the compilation for the `map` operation to produce `n` copies of the hardware we generate above and generate a control program that looks like this:
```
par {
  while le_b0.out with cond_b0 { seq { eval_body_b0; incr_idx_b0; } }
  while le_b1.out with cond_b1 { seq { eval_body_b1; incr_idx_b1; } }
  while le_b2.out with cond_b2 { seq { eval_body_b2; incr_idx_b2; } }
  while le_b3.out with cond_b3 { seq { eval_body_b3; incr_idx_b3; } }
}
```

The [`par` operator][lf-par] executes all the loops in parallel.
The [full implementation][impl] shows the necessary code to accomplish this which simply creates an outer loop to generate distinct hardware for each copy of the loop.

## Conclusion

Hopefully this should be enough to get you started with writing your own MrXL compiler. Some more follow-up tasks you could try if you're interested:
- Read the code for compiling `reduce` statements and extend to support parallel reductions using [reduction trees][reduc-trees].
- Implement code generation that allows memories that differ from one another in size.
- Implement complex function body expressions. We only support binary operations with two operands, like `a + 5`.
- Add a new `filter` operation to MrXL.

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
[lf-while]: ../lang/ref.md#while
[lf-comb-group]: ../lang/ref.md#comb-group-definitions
[lf-par]: ../lang/ref.md#par
[impl]: https://github.com/cucapra/calyx/blob/master/frontends/mrxl/mrxl/gen_futil.py
[reduc-trees]: http://www.cs.ucr.edu/~nael/217-f15/lectures/217-lec10.pdf
[builder-ex]: https://github.com/cucapra/calyx/blob/master/calyx-py/test/builder_example.py