# Attributes

Calyx has an attribute system that allows information to be associated with
every basic Calyx construct. This information can then be used to optimize the program
or change how the program is compiled.

Attributes can decorate lots of things in Calyx: components, groups, cells, ports, and control statements.
The syntax looks like `name<"attr"=value>` for components and groups or `@attr(value)` for other constructs.
Attributes always map keys to values.
Because it's common to have a "Boolean" attribute that always maps to the value 1, the syntax `@attr` is a shorthand for `@attr(1)`.

Here is the syntax for attributes in different parts of the AST:

#### **Component and Port Attributes**
```
component main<"static"=10>(@go go: 1) -> (@done done: 1) {
 ...
}
```

#### **Cell Attributes**
```
cells {
  @external mem = std_mem_d1(32, 8, 4);
  reg = std_reg(32);
  ...
}
```

#### **Group Attributes**
```
group cond<"static"=1> {
  ...
}
```

#### **Control Attributes**
```
control {
  @static(3) seq {
    @static(1) A;
    @static(2) B;
  }
}
```

## Meaning of Attributes
### `external`
The `external` attribute has meaning when it is attached to a cell.
It has two meanings:
1. If the `externalize` pass is enabled, the cell is turned into an "external"
   cell by exposing all its ports through the current component and rewriting
   assignments to the use the ports. See the documentation on
   See [externalize](https://capra.cs.cornell.edu/docs/calyx/source/calyx/passes/struct.Externalize.html "Externalize Pass") for more information.
2. If the cell is a memory and has an `external` attribute on it, the verilog backend (`-b verilog`) generates code to read `<cell_name>.dat` to initialize the memory state and dumps out its final value after execution.

### `static(n)`
Can be attached to components, groups, and control statements. They indicate how
many cycles a component, group, or control statement will take to run and are used
by `-p static-timing` to generate more efficient control FSMs.

### `go`, `done`, and `reset`
These three ports are part of the interface to Calyx components.
They are the mechanism for how an "outer" component invokes an "inner" cell that it contains.

The `go` and `done` attributes are, in particular, used by the `infer-static-timing` pass to configure which ports are used like
`go` and `done` signals.
Along with the `static(n)` attribute, this allows the pass to calculate when
a particular done signal of a primitive will be high.

### `stable`
Applied to port definitions of primitives and components. The intended semantics
are that after invoking the component, the value on the port remains latched
till the next invocation.

For example
```
cells {
  m = std_mult_pipe(32);
}
wires {
  group use_m_out { // uses m.out }
}
control {
  invoke m(left = 32'd10, right = 32'd4)();
  use_m_out;
}
```

The value of `m.out` in `use_m_out` will be `32'd40`.

This annotation is currently used by the primitives library and the Dahlia
frontend and is not checked by any pass.

### `share`
Can be attached to a component and indicates that a component can be shared
across groups. This is used by the `-p resource-sharing` to decide which components
can be shared.

### `bound(n)`
Used in `infer-static-timing` and `static-timing` when the number of iterations
of a `While` control is known statically, as indicated by `n`.

### `generated`
Added by [`ir::Builder`][builder] to denote that the cell was added by a pass.

### `clk`
Marks the special clock signal inserted by the `clk-insertion` pass, which helps with lowering to RTL languages that require an explicit clock.

### `write_together(n)`
Used by the `papercut` pass.
Defines a group `n` of signals that all must be driven together:
```
primitive std_mem_d2<"static"=1>[WIDTH, D0_SIZE, D1_SIZE, D0_IDX_SIZE, D1_IDX_SIZE](
  @write_together(2) addr0: D0_IDX_SIZE,
  @write_together(2) addr1: D1_IDX_SIZE,
  @write_together(1) write_data: WIDTH,
  @write_together(1) @go write_en: 1,
  ...
) -> (...);
```

This defines two groups.
The first group requires that `write_en` and `write_data` signals together
while the second requires that `addr0` and `addr1` are driven together.

Note that `@write_together` specifications cannot encode implication of the
form "if port `x` is driven then `y` should be driven".

### `read_together(n)`
Used by the `papercut` pass.
Defines a group `n` in which when the read port is used then all the write
ports must be driven as well.
```
primitive std_mem_d1<"static"=1>[WIDTH, SIZE, IDX_SIZE](
  @read_together(1) addr0: IDX_SIZE, ...
) -> (
  @read_together(1) read_data: WIDTH, ...
);
```

This requires that when `read_data` is used then `addr0` must be driven.
Note that each group must have exactly one output port in it.


[builder]: https://capra.cs.cornell.edu/docs/calyx/source/calyx/ir/struct.Builder.html
