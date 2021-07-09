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

### `share`
Can be attached to a component and indicates that a component can be shared
across groups. This is used by the `-p resource-sharing` to decide which components
can be shared.

### `bound(n)`
Used in `infer-static-timing` and `static-timing` when the number of iterations 
of a `While` control is known statically, as indicated by `n`.

### `generated`
Added by [`ir::Builder`][builder] to denote that the cell was added by a pass.

[builder]: https://capra.cs.cornell.edu/docs/calyx/source/calyx/ir/struct.Builder.html
