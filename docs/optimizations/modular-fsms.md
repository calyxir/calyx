By default, the Calyx compiler implements its control flow abstractions using
compiler-generated groups. During the lowering process, these groups eventually
become assignments, inlined with and largely indistinguishable from assignments 
provided by the user themselves.

Alternatively, the Calyx compiler supports separation of the data and control 
paths. That is, groups and assignments specified by the user can be can be 
implemented distinctly from the hardware that schedules their execution.

## Compiler Arguments
The following pass specifications will compile your design with modularized finite state machines (FSMs). Importantly, these new specifications cannot be used in addition to the default passes (i.e. `-p pre-opt -p compile`); they must entirely replace the default pipeline, pre-lowering and pre-backend.

```
-p fsm-opt -x tdcc:infer-fsms
```

### Details about `fsm-opt` and `infer-fsms`

`fsm-opt` is a set of passes that works as a direct substitution for the default pipeline `-p pre-opt -p compile`. While many passes present in the default pipeline also exist in `fsm-opt`, the tangible difference is in its representation of control
flow. Instead of using groups to schedule execution of other groups and assignments, `fsm-opt` relies on a distinct internal representation for FSMs. Eventually, 
when emitting RTL programs, each FSM IR construct is compiled into its own module and will provide control signals to the component (i.e. data path) module from afar.

While using `fsm-opt`, modularized implementation of `static` control flow is enabled
by default. In order to get a modularized implementation for dynamic control,
we need to pass the argument `infer-fsms` into the top-down-dynamic-control (TDCC)
pass.

The FSM IR construct is flexible enough to implement all existing Calyx control
abstractions. At every state of an FSM, the compiler must generate a list of assignments to activate at that state, along with a list of possible transitions.

### Example
Consider the following control program, where each group enable is dynamic.
```
control {
    seq {
      A; 
      B; 
      C;
    }
  }
```
Using the compiler arguments given above, you should see a result similar to the following:
```
wires {

  //
  // You might see some groups here. But then, ...
  //

  fsm seq_fsm {
      0 : {} => {
        seq_fsm[start] -> 1,
        default -> 0,
      },
      1 : {
        A[go] = !A[done] ? 1'd1;
      } => {
        A[done] -> 2,
        default -> 1,
      },
      2 : {
        B[go] = !B[done] ? 1'd1;
      } => {
        B[done] -> 3,
        default -> 2,
      },
      3 : {
        C[go] = !C[done] ? 1'd1;
      } => {
        C[done] -> 4,
        default -> 3,
      },
      4 : {
        seq_fsm[done] = 1'd1;
      } => 0,
  }
}

control {
  seq_fsm;
}
```

### Supported Backends
As of now, modularized FSMs are only supported for the Verilog backend. 
