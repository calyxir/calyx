# FuTIL
Fuse Temportal Intermediate Language  
An intermediate language for [Fuse](https://github.com/cucapra/seashell).

## Difference between Control and Structure
Structure consists of all static components of a circuit. This consists
of things like feeding inputs into an adder and getting the output. Another
way to think about the structure is as a graph representing the computation.
All structure is physically realizable.

Control gives you a way of expressing the more dynamic behavior of a graph, conditional
activation of sub-circuits, enforcing a logical notion of time (e.g subcircuit A runs
and only then subcircuit B runs), and repeated activation of a circuit (loops).

## Goal
All behavior is representable in the structure. However, when control is expressed
in structure, all the high level logical structure of the program becomes obscured.

In this light, you can think of representions of circuits as existing on a continuum
from all structure to all control. The goal of Futil is to be able to express a range
of this continuum to allow for the gradual lowering of control into structure without
ever leaving the IL.

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
the side-effect is not triggered when a circuit is not active.

## Syntax
You can define new modules as follows:
```racket
(define/module name ((in1 : 32) (in2 : 32)) (out1 : 16)
  (structure body ...)
  control body ...
  ...)
```

There are 5 kinds of statements that can go in the structure body:
- Module instantiation: `[name = new module]`
- Port connections: `[in1 -> out1]`
- Port splitting: `[name1 & name2 = split 16 in1]`
- Constants: `[const n : width -> other]` 
- Control Points: `[control a = name1, name2, ..., nameN]`
(constants can only show up on the left side of arrows)

The control body is optional. Syntax not yet implemented.
