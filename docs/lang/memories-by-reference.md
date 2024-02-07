# Passing Cells by Reference

One question that may arise when using Calyx as a backend is how to
pass a cell "by reference" between components. In C++, this might look like:
```C++
#include <array>
#include <cstdint>

// Adds one to the first element in `v`.
void add_one(std::array<uint32_t, 1>& v) {
  v[0] = v[0] + 1;
}

int main() {
  std::array<uint32_t, 1> x = { 0 };
  add_one(x); // The value at x[0] is now 1.
}
```

In Calyx, there are two steps to passing a cell by reference:
1. Define the component in a manner such that it can accept a cell by reference.
2. Pass the desired cell by reference.

When we say cell, we mean any cell, including memories of various dimensions and registers.

The language provides two ways of doing this.

## The Easy Way: `ref` Cells

Calyx uses the `ref` keyword to describe cells that are passed by reference:

```
component add_one() -> () {
  cells {
    ref mem = comb_mem_d1(32, 4, 3); // A memory passed by reference.
    ...
  }
  ...
}
```

This component defines `mem` as a memory that is passed by reference to the component.
Inside the component, we can use the cell as usual.

Next, to pass the memory to the component, we use the `invoke` syntax:
```
component add_one() -> () { ... }
component main() -> () {
  cells {
    A = comb_mem_d1(32, 4, 3); // A memory passed by reference.
    one = add_one();
    ...
  }
  wires { ... }
  control {
    invoke one[mem = A]()(); // pass A as the `mem` for this invocation.
  }
}
```

The Calyx compiler will correctly lower the `add_one` component and the `invoke` call such that the memory is passed by reference.
In fact, any cell can be passed by reference in a Calyx program.
Read the next section if you're curious about how this process is implemented.

### Multiple memories, multiple components

To understand the power of `ref` cells, let us work through an example.
We will study a relatively simple _arbitration logic_:
the invoker has six memories of size 4 each, but needs to pretend, sometimes simulatenously, that:
1. They are actually _two_ memories of size _12_ each.
2. They are actually _three_ memories of size _8_ each.


We will do up two components that are designed to receive memories by reference:

```
component wrap2(i: 32, j: 32) -> () {
  cells {
    // Six memories that will be passed by reference.
    ref mem1 = comb_mem_d1(32, 4, 32);
    // ...
    ref mem6 = comb_mem_d1(32, 4, 32);
    // An answer cell, also passed by reference.
    ref ans = comb_mem_d1(32, 1, 32);
  }
  wires { ... }
  control { ... }
}
```
and
```
component wrap3(i: 32, j: 32) -> () {
  cells {
    // Six memories that will be passed by reference.
    ref mem1 = comb_mem_d1(32, 4, 32);
    // ...
    ref mem6 = comb_mem_d1(32, 4, 32);
    // An answer cell, also passed by reference.
    ref ans = comb_mem_d1(32, 1, 32);
  }
  wires { ... }
  control { ... }
}
```

That is, they have the same signature including `input` ports, `output` ports, and `ref` cells.
We have elided the logic, but feel free to explore the [source code][arbiter_6.futil].

Now the invoker has six locally defined memories.
By passing these memories to the components above, the invoker is able to wrap the same six memories two different ways, and then maintain two different fictional indexing systems at the same time.

```
component main() -> () {
  cells {
    // Six memories that will pass by reference.
    @external A = comb_mem_d1(32, 4, 32);
    //...
    @external F = comb_mem_d1(32, 4, 32);

    // Two answer cells that we will also pass.
    @external out2 = comb_mem_d1(32, 1, 32);
    @external out3 = comb_mem_d1(32, 1, 32);

    // Preparing to invoke the components above.
    together2 = wrap2();
    together3 = wrap3();
  }

  wires {
  }

  control {
    seq {
      invoke together2[mem1=A, mem2=B, mem3=C, mem4=D, mem5=E, mem6=F, ans=out2](i=32'd1, j=32'd11)();
      invoke together3[mem1=A, mem2=B, mem3=C, mem4=D, mem5=E, mem6=F, ans=out3](i=32'd2, j=32'd7)();
    }
  }
}
```

Observe: when "wrapped" into two chunks, \\( 0 \le i < 2 \\) and \\( 0 \le j < 12 \\); when wrapped into three chunks, \\( 0 \le i < 3 \\) and \\( 0 \le j < 8 \\).


## The Hard Way: Without `ref` Cells

> Proceed with caution. We recommend using the `ref` syntax in almost all cases since it enables the compiler to perform more optimizations.

If we wish not to use `ref` cells, we can leverage the usual `input` and `output` ports to establish a call-by-reference-esque relationship between the calling and called components.
In fact, the Calyx compiler takes `ref` cells as descibed above and lowers them into code of the style described here.

Let us walk through an example.

### Worked example: `mem_cpy`

In the C++ code above, we've constructed an "l-value reference" to the array,
which essentially means we can both read and write from `x` in the function
`add_one`.

Now, let's allow similar functionality at the Calyx IR level.
We define a new component named `add_one` which represents the function
above. However, we also need to include the correct ports to both read
and write to `x`:

|  Read from `x` | Write to `x`  |
|----------------|---------------|
| read_data      | done          |
| address ports  | write_data    |
|                | write_en      |
|                | address ports |

Since we're both reading and writing from `x`, we'll
include the union of the columns above:
```
{{#include ../../examples/futil/memory-by-reference/memory-by-reference.futil:component_ports}}
```

One tricky thing to note is where the ports belong, i.e. should it be
an input port or an output port of the component? The way to reason about this
is to ask whether we want to receive signal from or send signal to the given wire. For example,
with `read_data`, we will always be receiving signal from it, so it should be an input port.
Conversely, address ports are used to mark where in memory we want to access,
so those are used as output ports.

We then simply use the given ports to both read and write to the memory passed
by reference. Note that we've split up the read and write to memory `x` in separate groups,
to ensure we can schedule them sequentially in the execution flow.
We're also using the exposed ports of the memory through the component interface rather than,
say, `x.write_data`.
```
{{#include ../../examples/futil/memory-by-reference/memory-by-reference.futil:wires}}
```

Bringing everything back together, the `add_one` component is written accordingly:
```
{{#include ../../examples/futil/memory-by-reference/memory-by-reference.futil:component}}
```

The final step is creating a `main` component from which the original component
will be invoked. In this step, it is important to hook up the proper wires in the
call to `invoke` to the corresponding memory you'd like to read and/or write to:
```
{{#include ../../examples/futil/memory-by-reference/memory-by-reference.futil:invoke}}
```

This gives us the `main` component:
```
{{#include ../../examples/futil/memory-by-reference/memory-by-reference.futil:main}}
```

To see this example simulated, run the command:
```
fud e examples/futil/memory-by-reference/memory-by-reference.futil --to dat \
-s verilog.data examples/futil/memory-by-reference/memory-by-reference.futil.data
```

### Multi-dimensional Memories
Not much changes for multi-dimensional arrays. The only additional step is adding
the corresponding address ports. For example, a 2-dimensional memory will require address ports
`addr0` and `addr1`. More generally, an `N`-dimensional memory will require address ports
`addr0`, ..., `addr(N-1)`.

### Multiple Memories
Similarly, multiple memories will just require the ports to be passed for each of the given memories.
Here is an example of a memory copy (referred to as `mem_cpy` in the C language), with 1-dimensional memories of size 5:
```
{{#include ../../tests/correctness/invoke-memory.futil}}
```

[arbiter_6.futil]: https://github.com/calyxir/calyx/blob/master/calyx-py/test/arbiter_6.futil