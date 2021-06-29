# Attributes

Calyx has an attribute system that allows information to be associated with
every basic Calyx construct. This information can then be used to optimize the program
or change how the program is compiled.

Here is the syntax for attributes in different parts of the AST:
#### **Component and Port Attributes**
```
component main<"static"=10>(@go(1) go: 1) -> (@done(1) done: 1) {
 ...
}
```

#### **Cell Attributes**
```
cells {
  @external(1) mem = std_mem_d1(32, 8, 4);
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
### `external(1)`
The `external(1)` attribute has meaning when it is attached to a cell.
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

### `go(1)` and `done(1)`
Used by the `infer-static-timing` pass to configure which ports are used like
`go` and `done` signals.
Along with the `static(n)` attribute, this allows the pass to calculate when
a particular done signal of a primitive will be high.

### `share(1)`
Can be attached to a component and indicates that a component can be shared
across groups. This is used by the `-p resource-sharing` to decide which components
can be shared.

### `bound(n)`
Used in `infer-static-timing` and `static-timing` when the number of iterations 
of a `While` control is known statically, as indicated by `n`.
