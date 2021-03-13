# Calyx Language Tutorial

This tutorial will familiarize you with the Calyx language by writing a minimal program *by hand*.
Of course, the usual thing to do is to generate Calyx code from a DSL compiler frontend instead, but by writing a program manually, you can get familiar with all the basics in the language that you will need to do fancier things.


## Get Started

We will start with this skeleton, which just imports the standard library and defines an empty *component*, called `main`:

    import "primitives/std.lib";

    component main() -> () {
      cells {
      }

      wires {
      }

      control {
      }
    }

Put this in a file—you can call it `hello.futil`, for example.
(The `futil` file extension comes from an old name for Calyx.)

You can think of a component as a unit of Calyx code roughly analogous to a function: it encapsulates a logical unit of hardware structures along with their control.
Every component definition has three sections:

* `cells`: The hardware subcomponents that make up this component.
* `wires`: A set of guarded connections between components, possibly organized into *groups*.
* `control`: The imperative program that defines the component's execution schedule: i.e., when each group executes.

We'll fill these sections up minimally in the next sections.


## A Memory Cell

Let's turn our skeleton into a tiny, nearly no-op Calyx program.
We'll start by adding a memory component to the cells:

    cells {
      mem = std_mem_d1(32, 1, 1);
    }

This new line declares a new cell called `mem` and the primitive component `std_mem_d1` represents a 1D memory.
You can see the definition of `std_mem_d1`, and all the other standard components, in the `primitives/std.lib` library we imported.
This one has three parameters:
the data width (here, 32 bits),
the number of elements (just one),
and the width of the address port (one bit).

Next, we'll add some assignments (wires) to update the value in the memory.
Insert these lines to put a constant value into the memory:

    wires {
      mem.addr0 = 1'b0;
      mem.write_data = 32'd42;
      mem.write_en = 1'b1;
    }

These assignments refer to three *ports* on the memory component:
`addr0` (the address port),
`write_data` (the value we're putting into the memory), and
`write_en` (the *write enable* signal, telling the memory that it's time to do a write).
Constants like `32'd42` are Verilog-like literals that include the bit width (32), the base (`d` for decimal), and the value (42).

Assignments at the top level in the `wires` section, like these, are "continuous."
They always happen, without any need for `control` statements to orchestrate them.
We'll see later how to organize assignments into groups.


## Compile & Run

We can almost run this program!
But first, we need to provide it with data.
The Calyx infrastructure can provide data to programs from [JSON][] files.
So make a file called something like `hello.json` containing something along these lines:

    {
      "mem": {
        "data": [10],
        "format": {
          "numeric_type": "bitnum",
          "is_signed": false,
          "width": 32
        }
      }
    }

The `mem` key means we're providing the initial value for our memory called `mem`.
We have one (unsigned integer) data element, and we indicate the bit width (32 bits).

If you want to see how this Calyx program compiles to Verilog, here's the fud incantation you need:

    fud exec hello.futil --to verilog

Not terribly interesting!
However, one nice thing you can do with programs is execute them.

To run our program using [Verilator][], do this:

    fud exec hello.futil --to dat -s verilog.data hello.json

Using `--to dat` asks fud to run the program, and the extra `-s verilog.data <filename>` argument tells it where to find the input data.
Executing this program should print:

    {
      "mem": [
        42
      ]
    }

Meaning that, after the program finished, the final value in our memory was 42.

[json]: https://www.json.org/
[verilator]: https://www.veripool.org/wiki/verilator


## Add Control

Let's change our program to use an execution schedule.
To do that, we'll need to organize the assignments in the `wire` section into a named *group:*

    wires {
      group the_answer {
        mem.addr0 = 1'b0;
        mem.write_data = 32'd42;
        mem.write_en = 1'b1;
        the_answer[done] = mem.done;
      }
    }

We also need one extra line in the group: that assignment to `the_answer[done]`.
Here, we say that `the_answer`'s work is one once the update to `mem` has finished.
Calyx groups have *compilation holes* called `go` and `done` that the control program will use to orchestrate their execution.

The last thing we need is a control program.
Add one line to activate `the_answer` and then finish:

    control {
      the_answer;
    }

If you execute this program, it should do the same thing as the original group-free version: `mem` ends up with 42 in it.
But now we're controlling things with an execution schedule.

If you're curious to see how the Calyx compiler lowers this program to a Verilog-like structural form of Calyx, you can do this:

    fud exec hello.futil --to futil-lowered

Notably, you'll see `control {}` in the output, meaning that the compiler has eliminated all the control statements and replaced them with continuous assignments in `wires`.


## Add an Adder

The next step is to actually do some computation.
In this version of the program, we'll read a value from the memory, increment it, and store the updated value back to the memory.

First, we will add two components to the `cells` section:

    val = std_reg(32);
    add = std_add(32);

We make a register `val` and an integer adder `add`, both configured to work on 32-bit values.

Next, we'll create three groups in the `wires` section for the three steps we want to run: read, increment, and write back to the memory.
Let's start with the last step, which looks pretty similar to our `the_answer` group from above, except that the value comes from the `val` register instead of a constant:

    group write {
      mem.addr0 = 1'b0;
      mem.write_en = 1'b1;
      mem.write_data = val.out;
      write[done] = mem.done;
    }

Next, let's create a group `read` that moves the value from the memory to our register `val`:

    group read {
      mem.addr0 = 1'b0;
      val.in = mem.read_data;
      val.write_en = 1'b1;
      read[done] = val.done;
    }

Here, we use the memory's `read_data` port to get the initial value out.

Finally, we need a third group to add and update the value in the register:

    group upd {
      add.left = val.out;
      add.right = 32'd4;
      val.in = add.out;
      val.write_en = 1'b1;
      upd[done] = val.done;
    }

The `std_add` component from the standard library has two input ports, `left` and `right`, and a single output port, `out`, which we hook up to the register's `in` port.
This group adds a constant 4 to the register's value, updating it in place.
We can enable the `val` register with a constant 1 because the `std_add` component is *combinational*, meaning its results are ready "instantly" without the need to wait for a done signal.

We need to extend our control program to orchestrate the execution of the three groups.
We will need a `seq` statement to say we want to the three steps sequentially:

    seq {
      read;
      upd;
      write;
    }

Try running this program again.
The memory's initial value was 10, and its final value after execution should be 14.


## Iterate

Next, we'd like to run our little computation in a loop.
The idea is to use Calyx's `while` control construct, which works like this:

    while <value> with <group> {
      <body>
    }

A `while` loop runs the control statements in the body until `<value>`, which is some port on some component, becomes zero.
The `with <group>` bit means that we activate a given group in order to compute the condition value that determines whether the loop continues executing.

Let's run our memory-updating `seq` block in a while loop.
Change the control program to look like this:

    control {
      seq {
        init;
        while lt.out with cond {
          par {
            seq {
              read;
              upd;
              write;
            }
            incr;
          }
        }
      }
    }

This version uses `while`, the parallel composition construct `par`, and a few new groups we will need to define.
The idea is that we'll use a counter register to make this loop run a fixed number of times, like a `for` loop.
First, we have an outer `seq` that invokes an `init` group that we will write to set the counter to zero.
The `while` loop then uses a new group `cond`, and it will run while a signal `lt.out` remains nonzero: this signal will compute `counter < 8`.
The body of the loop runs our old `seq` block in parallel with a new `incr` group to increment the counter.

Let's add some cells to our component:

    counter = std_reg(32);
    add2 = std_add(32);
    lt = std_lt(32);

We'll need a new register, an adder to do the incrementing, and a less-than comparator.

We can use these raw materials to build the new groups we need: `init`, `incr`, and `cond`.
First, the `init` group is pretty simple:

    group init {
      counter.in = 32'd0;
      counter.write_en = 1'b1;
      init[done] = counter.done;
    }

This group just writes a zero into the counter and signals that it's done.
Next, the `incr` group adds one to the value in `counter` using `add2`:

    group incr {
      add2.left = counter.out;
      add2.right = 32'd1;
      counter.in = add2.out;
      counter.write_en = 1'b1;
      incr[done] = counter.done;
    }

And finally, `cond` uses our comparator `lt` to compute the signal we need for our `while` loop:

    group cond {
      lt.left = counter.out;
      lt.right = 32'd8;
      cond[done] = 1'b1;
    }

By comparing with 8, we should now be running our loop body 8 times.

Try running this program again.
The output should be the result of adding 4 to the initial value 8 times, so 10 + 8 × 4.
