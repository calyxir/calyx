# Attributes

Calyx has an attribute system that allows information to be associated with
every basic Calyx construct. This information can then be used to optimize the program
or change how the program is compiled.

Here is the syntax for attributes in different parts of the AST:
#### **Component and Port Attributes**
```
component main<"static"=10>(@go_port(1) go: 1) -> (@done_port(1) done: 1) {
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
When this attribute is present and the `-p external` pass is enabled,
Calyx will externalize the ports of the cell into the component interface.
This is useful for memories and for debugging signals.

See [externalize](https://capra.cs.cornell.edu/docs/calyx/source/calyx/passes/struct.Externalize.html "Externalize Pass")
for more information.

### `static(n)`
Can be attached to components, groups, and control statements. They indicate how
many cycles a component, group, or control statement will take to run and are used
by `-p static-timing` to generate more efficient control FSMs.

### `share(1)`
Can be attached to a component and indicates that a component can be shared
across groups. This is used by the `-p resource-sharing` to decide which components
can be shared.
