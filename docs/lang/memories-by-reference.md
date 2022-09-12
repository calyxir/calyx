# Passing Memories by Reference

One question that may arise when using Calyx as a backend is how to
pass a memory "by reference" between components. In C++, this might look like:
```
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

There are two steps to passing a memory by reference in Calyx:
1. Define the component in a manner such that it can accept a memory by reference.
2. Pass the desired memory by reference.

The language provides two ways to doing this.

## The Easy Way

Calyx uses the `ref` keyword to describe cells that are passed in by-reference:

```
component add_one() -> () {
  cells {
    ref mem = std_mem_d1(32, 4, 3); // A memory passed in by reference.
    ...
  }
  ...
}
```

This component define `mem` as a memory that is passed in by reference to the component.
Inside the component we can use the cell like any other cell in the program.

Next, to pass the memory to the component, we can use the `invoke` syntax:
```
component add_one() -> () { ... }
component main() -> () {
  cells {
    A = std_mem_d1(32, 4, 3); // A memory passed in by reference.
    one = add_one();
    ...
  }
  wires { ... }
  control {
    invoke one[mem = A]()(); // pass A as the `mem` for this invocation
  }
}
```

The Calyx compiler will correctly lower the `add_one` component and the `invoke` call such that the memory is passed in by-reference.
In fact, any cell can be passed in by-reference in a Calyx program.
Read the next section if you're curious about how this process is implemented.

## The Hard Way

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

## Multi-dimensional Memories
Not much changes for multi-dimensional arrays. The only additional step is adding
the corresponding address ports. For example, a 2-dimensional memory will require address ports
`addr0` and `addr1`. More generally, an `N`-dimensional memory will require address ports
`addr0`, ..., `addr(N-1)`.

## Multiple Memories
Similarly, multiple memories will just require the ports to be passed for each of the given memories.
Here is an example of a memory copy (referred to as `mem_cpy` in the C language), with 1-dimensional memories of size 5:
```
{{#include ../../tests/correctness/invoke-memory.futil}}
```
