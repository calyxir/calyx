# FuTIL
Fuse Temportal Intermediate Language  
An intermediate language for [Fuse](https://github.com/cucapra/seashell).

## Difference between Control and Structure
Structure consists of all static components of a circuit. This consists
of things like feeding inputs into an adder and getting the output. Another
way to think about the structure is as a graph representing the computation.
All structure is physically realizable. (correlates to [combinational logic](https://en.wikipedia.org/wiki/Combinational_logic))

Control gives you a way of expressing the more dynamic behavior of a graph. Such as conditional
activation of sub-circuits, enforcing a logical notion of time (e.g subcircuit A runs
and only then subcircuit B runs), and repeated activation of a circuit (loops).
(maybe a nice way of lifting the combinational logic to [sequential logic](https://en.wikipedia.org/wiki/Sequential_logic) 
without having explicit clocks and flip-flops, but working at a more logical level).

## Goal
All behavior is representable in the structure (because otherwise it could never be hardware). 
However, when control is expressed in structure, all the high level logical structure 
of the program becomes obscured. 

The specification of a circuit exists on a continuum between all control and all structure.
THe goal of Futil is to be able to express specifications of circuits on a range of
this continuum so that you can start off with a representation that is mostly control (and close
to the source Fuse program) and then gradually lower the specification into mostly structure.

## Details
The basic unit of Futil is a `module`. Modules carry around structure and optionally control.
You define a module with `define/module`. There is an elaboration on the syntax below.

### Specifying Structure
At the beginning of a module definition, you define a list of input port and output ports
along with corresponding names and widths. In the body of the structure definition, you
can instantiate other modules, and connect ports together. This lets you define the
a computation graph. You can also declare sets of submodules as control points.
A control point is a place where the control is allowed to act.

### Specifying Control
The control code has access to the control points defined in the structure. In the control
section, you can "activate" the sub-circuit contained in a control point. If a sub-circuit
is activated, then values flow through it normally. If a circuit is not activated, then values
are stopped before entering the sub-circuit. This ensures that if a sub-circuit has a side-effect,
the side-effect is not triggered when a circuit is not active. In addition to activation, there
are ways to direct a logical flow of time. You can have sequentional compostion, 
`(A --- B)`, and parallel composition, `(A ; B)` that let you express whether two sub-circuits should
be activated in parallel or whether the second activation shouldn't be triggered until after the
first completes. There are also conditionals of the form `if A then B else C` which activates `B` or `C`
depending on the result of activating `A`. (maybe `A` shouldn't be an activation but rather a variable storing
the result of `A`? Idk yet how variables should fit into this). Finally there are loops which let you
specify that a sub-circuit should be activated repeatedly over time.

## Syntax
You can define new modules as follows:
```racket
(define/module name ((in1 : 32) (in2 : 32)) (out1 : 16)
  (structure body ...)
  control body ...)
```

There are 4 kinds of statements that can go in the structure body:
 - Module instantiation: `[name = new module]`
 - Port connections: `[in1 -> out1]`
 - Port splitting: `[name1 & name2 = split 16 in1]`
 - Constants: `[const name n : width -> other]` 
   (constants can only show up on the left side of arrows)

The control body is optional. There are two different ways to compose
stmts: parallel composition and sequential composition.
Consider `[(stmt-1) ...] [(stmt-2) ...] ...`
The square brackets denote sequential composition while the stmts
inside the square brackets denote parallel composition. 

There are 4 kinds of control statements.
 - Submodule deactivation: `(a b ...)`. This means deactivate
 submodules a, b, ...
 - Valued condition: `(if (name port) (tbranch stmts ...) (fbranch stmts ...))`
 If there is a value on the wire `(name . port)`, then if the value is non-zero
 go into the true branch, otherwise go into the false branch. If there is no value
 on the wire, then this expression does nothing.
 - Enable condition: `(ifen (name port) (tbranch stmts ...) (fbranch stmts ...))`
 This conditional statement lets you check if a wire is enabled or disabled. If
 `(name . port)` has a value, then go into the true branch, otherwise go into the
 false branch.
 - While loop: `(while (name port) (body ...))`
 Equivalent to `[if (name port) ([body] [(while (name port) (body))]) ()]`
 Note that this uses a valued conditional rather than the enable condition.
