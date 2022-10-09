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

### `toplevel`
The entrypoint for the Calyx program. If no component has this attribute, then
the compiler looks for a component named `main`. If neither is found, the
compiler errors out.

### `go`, `done`, `clk` and `reset`
These four ports are part of the interface to Calyx components.
These are automatically added by the parser if they are missing from the component definition.
`go` and `done` provide the mechanism for how an "outer" component invokes an "inner" cell that it contains.
`clk` and `reset` thread through the global clock and resetting signal in a design.

### `nointerface`
By default, interface ports are automatically added to a component by the parser if they are missing.
Adding this attribute disables this behavior.

### `external`
The `external` attribute has meaning when it is attached to a cell.
It has two meanings:
1. If the `externalize` pass is enabled, the cell is turned into an "external"
   cell by exposing all its ports through the current component and rewriting
   assignments to the use the ports. See the documentation on
   See [externalize](https://docs.calyxir.org/source/calyx/passes/struct.Externalize.html "Externalize Pass") for more information.
2. If the cell is a memory and has an `external` attribute on it, the verilog backend (`-b verilog`) generates code to read `<cell_name>.dat` to initialize the memory state and dumps out its final value after execution.

### `static(n)`
Can be attached to components, groups, and control statements. They indicate how
many cycles a component, group, or control statement will take to run and are used
by `-p static-timing` to generate more efficient control FSMs.

The `go` and `done` attributes are, in particular, used by the `infer-static-timing` pass to configure which ports are used like
`go` and `done` signals.
Along with the `static(n)` attribute, this allows the pass to calculate when
a particular done signal of a primitive will be high.

### `inline`
Used by the `inline` pass on cell definitions. Instructs the pass to completely
inline the instance into the parent component and replace all `invoke`s of the
instance with the control program of the instance.

### `stable`
Used by the `canonicalize` pass.
Only meaningful on output ports and states that their value is provided by
a sequential element and is therefore available outside combinational time.

For example, after invoking a multiplier, the value on its `out` port remains
latched till the next invocation.

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
frontend.

### `share`
Can be attached to a component and indicates that a component can be shared
across groups. This is used by the `-p cell-share` to decide which components
can be shared.

### `state_share`
Can be attached to a component and indicates that a component can be shared
across groups. Different than `share` since `state_share` components can have
internal state.
This is used by `-p cell-share` to decide which components can be shared.
Specifically, a component is state shareable if each write to
that component makes any previous writes to the component irrelevant.
The definition of a "write to a component" is an activiation of
the component's "go" port, followed by a read of the component's "done" port (in
other words, the read of a "done" port still counts as part of a "write" to the
component).
For `c1` and `c2`, instances of a state_shareable component:
instantiate `c1`                        instantiate `c2`
*any write to `c1`*                     *any write to `c2`*
*write value `v` to port `p` in `c1`*   *write value `v` to port `p` in `c2`*
`c1` and `c2` should be equal.

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

Used by `papercut` and `canonicalize`.
Defines a combinational path `n` between a set of an input ports and an output
port.
```
primitive std_mem_d1<"static"=1>[WIDTH, SIZE, IDX_SIZE](
  @read_together(1) addr0: IDX_SIZE, ...
) -> (
  @read_together(1) read_data: WIDTH, ...
);
```

This requires that when `read_data` is used then `addr0` must be driven.
Note that each group must have exactly one output port in it.

### `@data`

Marks a cell or a port as a *purely datapath* component, i.e., the output does not propagate into a guard or another control signal. See [this issue][datapath-components] for the full set of constraints.

When we have following two conditions:
1. An input port is marked with `@data` in the component definitions, and
2. The cell instance is marked as `@data`

The backend generate `'x` as the default value for the assignment to the port instead of `'0`. Additionally, if the port has exactly one assignment, the backend removes the guard entirely and produces a continuous assignment.

This represents the optimization:
```
in = g ? out : 'x
```
into:
```
in = out;
```
Since the value `'x` can be replaced with anything.


[datapath-components]: https://github.com/cucapra/calyx/issues/1169
[builder]: https://docs.calyxir.org/source/calyx/ir/struct.Builder.html
