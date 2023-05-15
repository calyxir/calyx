# Building a Frontend for Calyx

> This tutorial assumes that you have already worked through the [Calyx tutorial][calyx-tut]. You won't get very much out of it by itself!

In the [Calyx tutorial][calyx-tut] you wrote Calyx by hand.
That is (probably) a good way to build character, but it's no way to live.
In practice, you want a *frontend* that *compiles* to Calyx.

This allows you to:
1. Support domain-specific commands that Calyx does not.
2. Automatically generate some of the kludge that Calyx requires.

In this tutorial, we're going to learn all about this by building a compiler for a toy language.
Meet MrXL.

# MrXL Overview

MrXL lets you define arrays (TK, after https://github.com/cucapra/calyx/issues/1459 lands: "and registers") and then perform `map` and `reduce` operations.

## A tiny example

Here's a MrXL program in all its glory:

```
{{#include ../../frontends/mrxl/test/sos.mrxl}}
```

The program is short enough for us to pick apart line by line:
1. We specify an array, `avec`, which will have four integers. The `input` keyword means that an external harness will populate the array.
2. We specify another array, `sos`, which will also have four integers. (TK: this will change to `output sos: int` after https://github.com/cucapra/calyx/issues/1459 lands, so the copy will become lighter: "We specify `sos`, a register.") The `output` keyword means that we will populate `sos` in our program.
3. The `map` operation gets the values of `avec` and raises each to the second power. We stash the result in a new array, `squares`. The number `1` denotes a *parallelism factor* of 1, meaning that the operation is performed sequentially. We will improve this shortly.
4. The `reduce` operation walks over `squares` and accumulates the result into an array. (TK: "a register"). The parallelism factor is again `1`.


## Running our example

Let's run this program.

To begin, [install the MrXL command line tool][mrxldocs-install].
(TK: I'd kinda like to inline those instructions here, so that there is less jumping around between files.
I want folks to just install and get back here so we can interpret together, but there's a risk that folks will just begin to follow the other guide and go all the way to Verilog there.
Also, the MrXL tutorial has a different running example...)

Now change directories to `calyx/frontends/mrxl` and run
```
mrxl test/sos.mrxl --data test/sos.mrxl.data --interpret
```

Why `42`? Because we populated `avec` with
```json
{{#include ../../frontends/mrxl/test/sos.mrxl.data}}
```
(TK: the file is gruesome right now, but once https://github.com/cucapra/calyx/issues/1450#issuecomment-1546757549 lands it'll look much nicer, to the point that it'll flow okay.)

and $0^2 + 1^2 + 4^2 + 5^2 = 42$.

Still not impressed?
Consider the Calyx code that we _didn't need to write_:

```
{{#include ../../frontends/mrxl/test/sos.calyx}}
```

# Breaking it Down

We mentioned two reasons to build your own frontend: the reduction of kludge, and the addition of domain-specific commands.
Let's go over how MrXL showcases these benefits.

## Lighter data files

(TK: basically a summary of what it took to close https://github.com/cucapra/calyx/issues/1450. 5-7 mins for Susan?)

You may have noticed that the data files that we pass to MrXL programs are lighter-weight than those we pass to Calyx programs.
They are lighter in two ways.

### A simpler data format

MrXL supports only a few kinds of data, meaning that interesting pieces of Calyx-native data turn into "just boilerplate" in MrXL-native data.
We can lighten MrXL-native data files and write a [`fud`][fud] pass to tack this information on, thus creating legal Calyx-native data files.

### Memory banking

Consider a small variation on our running example.
The noteworthy change is the parallelism factor of the `map` operation.

```
{{#include ../../frontends/mrxl/test/squares.mrxl}}
```

Run this with
```
mrxl test/squares.mrxl --data test/squares.mrxl.data --interpret
```

To take advantage of the parallelism in the program, the MrXL compiler assumes that the input memory `avec` is, in fact, split into two *banks*, named `avec_b0` and `avec_b1`.
We will discuss this further shortly, but for now it suffices to convince oneself that this is needed.

While we supplied `avec`'s values as a straightforward array:

```json
{{#include ../../frontends/mrxl/test/squares.mrxl.data}}
```

under the hood, the *compiled version* of `squares.mrxl` came to expect something like:

```json
{{#include ../../frontends/mrxl/test/squares.mrxl.banked.data}}
```

Though nontrivial, this memory banking can also be handled automatically using `fud`; all the necessary information is in the MrXL source program.


## Compiling MrXL into Calyx

Our frontends can support new commands that are relevant to the domain of interest, so long as those commands can themselves be compiled into Calyx.
We will now study [the MrXL-to-Calyx compiler][impl], written in Python.

We have placed a few simplifying restrictions on MrXL programs:
1. Every array in a MrXL program has the same length.
2. Every integer in our generated hardware is 32 bits long.
3. The bodies of `map` and `reduce` operations must be `+` or `*` operations involving array elements or integers.
4. If repeated `map`/`reduce` operations are performed on the same memory, each of those operations must have the same parallelism factor.

These restrictions can be lifted or relaxed via commensurate changes to the compiler.

The compilation process breaks into two steps:
1. Parsing MrXL into a representation we can process in Python.
1. Generating Calyx code.

### Parsing MrXL into an AST

To start, we'll parse the MrXL program into a Python AST representation. We chose to represent [AST][astcode] nodes with Python `dataclass`.
A program is a sequence of array declarations followed by computation statements:
```python
{{#include ../../frontends/mrxl/mrxl/ast.py:prog}}
```

`Decl` nodes correspond to array declarations such as `input avec: int[4]`, and carry data about whether the array is an `input` or `output` array, its name, and its type:

```python
{{#include ../../frontends/mrxl/mrxl/ast.py:decl}}
```

`Stmt` nodes represent statements such as `sos := reduce 1 (acc, i <- squares) 0 { acc + i }`, and contain more nested nodes representing the function-header and -body, and the type of operation.

```python
{{#include ../../frontends/mrxl/mrxl/ast.py:stmt}}
```

We elide further details, but point you to the [AST][mrxl-ast], which defines all the nodes we need to represent a MrXL program.


### Generating Calyx Code

As you know, the skeleton of a Calyx program has three sections:

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

#### Calyx-Embedded DSL

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
At a high level, we want to generate the following pieces of hardware:
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

This code instantiates an adder or a multiplier depending on the computation needed using the `expr_to_port` helper function:
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

### Adding Parallelization

MrXL allows us to parallelize your `map` and `reduce` operations.
Let's revisit the `map` example from earlier:

```
{{#include ../../frontends/mrxl/test/sos.mrxl}}
```

The banking factor `2` specifies that two copies of the loop bodies should be executed in parallel.
Our implementation already creates [memory banks](#decl-nodes) to allow for parallel accesses.
At a high-level, we can change the compilation for the `map` operation to produce `n` copies of the hardware we generate above and generate a control program that looks like this:
```
par {
  while le_b0.out with cond_b0 { seq { eval_body_b0; incr_idx_b0; } }
  while le_b1.out with cond_b1 { seq { eval_body_b1; incr_idx_b1; } }
}
```

The [`par` operator][lf-par] executes all the loops in parallel.
The [full implementation][impl] shows the necessary code to accomplish this which simply creates an outer loop to generate distinct hardware for each copy of the loop.

# Further Steps

Congratulations, you know as much about MrXL as we do!
The small size of the language makes it a nice sandbox for you to play in.
Some fun things you could try:
1. We don't yet implement reduction in parallel. Extend the compiler to allow this.
2. We require that all arrays be the same size. Lift this restriction.
3. Permit complex expressions in the bodies of `map` and `reduce`.
4. Support a new `filter` operation in MrXL.
5. We require that repeated computations on the same array have the same banking factor. Lift this restriction.

[astcode]: https://github.com/cucapra/calyx/blob/mrxl/mrxl/mrxl/ast.py
[mrxldocs-install]: https://docs.calyxir.org/frontends/mrxl.html#install
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
