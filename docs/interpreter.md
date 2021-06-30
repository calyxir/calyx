# The Calyx Interpreter

The experimental Calyx interpreter resides in the `interp/` directory of the
repository.

In order to run an example program, run:
```
cd interp/ && cargo run interp/tests/add_feeding.futil
```

The interpreter supports all Calyx programs--from high-level programs that make
heavy use of control operators, to fully lowered Calyx programs.
