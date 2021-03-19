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

In the code above, we've constructed an "l-value reference" to the array,
which essentially means we can both read and write from `x` in the
function `add_one`.

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
is to ask whether we want to receive from or send signal to the given wire. For example,
with `read_data`, we will always be receiving signal from it, so it should be an input port.
On the contrary, we'll be using address ports to mark where in the memory we want to be,
so those should be used as output ports.

We then simply use the given ports to both read and write to the memory passed
by reference. Note that we've split up the read and write to memory `x` in separate groups,
to ensure we can schedule them sequentially in the execution flow.
We're also using the exposed ports of the memory through the component interface rather than, 
say, `x.write_data`.
```
{{#include ../../examples/futil/memory-by-reference/memory-by-reference.futil:wires}}
```

Bringing everything back together, the `add_one` component then ends up being:
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
Not much changes for mult-dimensional arrays. The only additional step is adding
the corresponding address ports. For example, a 2-dimensional memory will require address ports
`addr0` and `addr1`. More generally, an `N`-dimensional memory will require address ports
`addr0`, ..., `addr(N-1)`.

## Multiple Memories
Similarly, multiple memories will just require the ports to be passed for the given memories.
Here is an example of a memory copy (referred to as `mem_cpy` in C), with memories of size 5:
```
{{#include ../../tests/correctness/invoke-memory.futil}}
```
