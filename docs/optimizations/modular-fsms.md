By default, the Calyx compiler implements its control flow abstractions using
compiler-generated groups. During the lowering process, these groups eventually
become assignments, inlined with and largely indistinguishable from assignments 
provided by the user themselves.

Alternatively, the Calyx compiler supports separation of the data and control 
paths. That is, groups and assignments specified by the user can be can be 
implemented distinctly from the hardware that schedules their execution.

## Compiler Arguments
The following pass specifications will compile your design with modularized finite state machines (FSMs). Importantly, these new specifications cannot be used in addition to the default passes (i.e. `-p all`); they must entirely replace the default pipeline.

```
-p fsm-opt -x tdcc:infer-fsms -p lower
```

### Details about `fsm-opt` and `infer-fsms`

`fsm-opt` is a set of passes that works as a direct substitution for the default pipeline `-p pre-opt -p compile`. While many passes present in the default pipeline also exist in `fsm-opt`, the tangible difference is in its representation of control
flow. Instead of using groups to schedule execution of other groups and assignments, `fsm-opt` relies on a distinct internal representation for FSMs. Eventually, 
when emitting RTL programs, each FSM IR construct is compiled into its own module and will provide control signals to the component (i.e. data path) module from afar.

While using `fsm-opt`, modularized implementation of `static` control flow is enabled
by default. In order to get a modularized implementation for dynamic control,
we need to pass the argument `infer-fsms` into the top-down-dynamic-control (TDCC)
pass. 

### Supported Backends
As of now, modularized FSMs are only supported for the Verilog backend. 