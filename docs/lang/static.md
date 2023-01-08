# Static Timing

By default, Calyx programs use a *latency-insensitive* model of computation.
This means that the compiler does not track the number of cycles it takes to perform a computation or run a control operator
In general, latency-insensitivity makes it easier to compose programs together and gives the compiler freedom to schedule operators however it wants.
However, the generated hardware to schedule the execution may not be efficient–especially if the program can take advantage of the *latency* information.

More crucially, however, it is impossible for *latency-insensitive* programs to interact with *latency-sensitive* hardware implemented in RTL.

Calyx uses the `@static` attribute to provide latency information to various constructs and provides strong guarantees about the generated programs.

## Guarantees

## Tricks & Tips

### Delay by `n` cycles

Sometimes it can be useful to delay the execution of a group by `n` cycles:
```
seq {
  @static(1) a; // Run in first cycle
  ???
  @static(2) b; // Run in the 10th cycle
}
```

A simple trick to achieve this is adding an empty group with `@static(n)` attribute on it:
```
cell {
  r = @std_reg(0);
}
@static(9) group delay_9 {
  delay_9[done] = r.out; // Don't use r.done here
}
seq {
    @static(1) a; // Run in first cycle
    @static(9) delay_9;
    @static(2) b; // Run in the 10th cycle
}
```

The static compilation pass `tdst` will never attempt to use the `delay_9`'s `done` condition and since there are no assignments in the group, it'll not generate any additional hardware.
