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
a computation graph.

### Specifying Control
The control code has the power to define logical time steps, conditionally deactivate submodules
based on the value on a wire, and specify loops. 

## Semantics
This is a informal description of the semantics of FuTIL.

In FuTIL, modules are built out of smaller submodules. Each submodule exposes a `procedure`
which represents some computation.

### Submodule Activation
Submodules are active by default. When a module is active, the values on it's input
wires are passed to the submodule's procedure as arguments. The output of the procedure
are put on the wires coming out of the submodule. The execution of all submodules are atomic
which means that even when a submodule defines time steps internally, externally they are
executed in a single time step. When a module is deactived, the values on it's input
wires are not passed to the submodule's procedure. Additionally, the output wires are disabled.

### Memory
This provides a mechanism for a module to store state. In a module definition, there is a flag
called `mode` (bad name). Currently this flag is impossible to set in FuTIL; it can only be
set when creating modules in `racket`. There is a primitive called `comp/reg` that has this
flag set and can be used as a register. When the flag is set, all enabled outputs of the module
are saved during the computation. If the module outputs a new value, then this value is written
into memory. If the module has a disabled output, then the value in memory is used for that output.
If the module is inactive, the outputs are disabled as normal.

Each module recursively keeps track of all of it's submodule's memory. This allows the creation of
a module that outputs distinct values given the same input.

### Composition
There are two different ways to compose different control statements together: sequential and parallel.
Let `a` and `b` be statements, then `[a] [b]` executes `a`, then with the resulting state and memory
of the module, executes `b`.

Let `a` and `b` be statements and `st` and `mem` be the current state and memory of the module, 
then `[a b ...]` evaluates to (approximately) `(merge (step a st mem) (step b st mem) ...)` where
`step` is the evaluation function and `merge` merges states and memories.

`merge` is defined on a per-wire basis. Let state by a function from wires to values.
Given states `st0` and `st1`, the output for each wire is defined according to the following
table. `#f` signifies that a wire is disabled.

| st0 | st1 | out         |
|-----|-----|-------------|
| #f  | a   | a           |
| a   | #f  | a           |
| a   | a   | a           |
| a   | b   | !! error !! |
| #f  | #f  | #f          |

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
 Equivalent to `[(if (name port) ([body] [(while (name port) (body))]) ())]`
 Note that this uses a valued conditional rather than the enable condition.

## Primitives
For all computational primitives, if one or more of the input wires is disabled, the 
output is disabled.

| name             | ins                  | outs | description                              |
|------------------|----------------------|------|------------------------------------------|
| `comp/id`        | in                   | out  | `out = in`                               |
| `comp/reg`       | in                   | out  | `out = in (also has memory bit set)`     |
| `comp/add`       | left, right          | out  | `out = left + right`                     |
| `comp/trunc-sub` | left, right          | out  | `out = max(left - right, 0)`             |
| `comp/sub`       | left, right          | out  | `out = left - right`                     |
| `comp/mult`      | left, right          | out  | `out = left * right`                     |
| `comp/div`       | left, right          | out  | `out = left / right`                     |
| `comp/and`       | left, right          | out  | `out = left & right` (bitwise)           |
| `comp/or`        | left, right          | out  | `out = left &#124; right` (bitwise)      |
| `comp/xor`       | left, right          | out  | `out = left ^ right` (bitwise)           |
| `magic/mux`      | left, right, control | out  | `out = if (control = 1) left else right` |

## Vizualization
There is a function `compute` which takes in a module, and a list of inputs and produces
a list of outputs (as well as some other information).
For example `(compute (comp/add) '((left . 10) (right . 10)))` computes the sum of 10 and 10.
You can visualize the results of a computation with by using the function `plot-compute` instead.
The arguments are the same.

## Examples
Building multiplication out of addition. First we need a way of counting down so that
we can do something `n` times.

```racket
(define/module counter ((in : 32)) ((out : 32))
  ([sub = new comp/sub]
   [reg = new comp/reg]
   [in -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> reg @ in]
   [reg @ out -> sub @ left]
   [reg @ out -> out])
  [(ifen (in inf#)
         ([])
         ([(in)]))])
```

Then, the acutal implementation of multiplication using a while loop
and addition. The `viz` submodule is not actually necessary. It just
lets you see the value coming out of counter in the pictures.

``` racket
(define/module mult ((a : 32) (b : 32)) ((out : 32))
  ([counter = new counter2.0]
   [add = new comp/add]
   [reg = new comp/reg]
   [viz = new comp/id]

   [b -> counter @ in]
   [counter @ out -> viz @ in]

   [const zero 0 : 32 -> add @ left]
   [a -> add @ right]
   [add @ out -> reg @ in]
   [reg @ out -> add @ left]
   [reg @ out -> out])
  []
  [(while (counter out) ([(b zero)]))])
```

Result of `(plot-compute (mult) '((a . 3) (b . 4))')`:
![Image 0 for mult example](imgs/mult-0.png)
![Image 1 for mult example](imgs/mult-1.png)

## Things that are broken / Things to do 
 - Port widths are not actually meaningful at the moment. You can put any number, string,
 or any racket value really on a wire. Please don't abuse this power for bad.
 Eventually you will only be able to put a n bit number on a n bit wire.
 - Umm...so about the multiplication example. It works nicely for small numbers, like 3 and 5,
 but once the numbers get bigger than 7 it starts giving you the wrong answers. I assure you that
 this is entirely intended behavior.
 - Tests are a good thing I hear
 - Figure out the proper way to merge memory in parallel composition
 - My vizualizer currently doesn't have animated animals carrying values along the wires
