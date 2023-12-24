# Experimental: Synchronization

Calyx's default semantics [do not admit][par-undef] any predictable form of language-level
synchronization in presence of parallelism.
We're currently experimenting with a suite of new primitives that add synchronization to the
language.

## `@sync` attribute

## Motivation

Consider the following control program in calyx:
```
{{#include ../../examples/sync/unsync-example/unsync-doc-example.futil:control}}
```

where groups `add_r_to_accm` and `incr_r` reads value and increments value in register `r`, respectively, as indicated by their names.

Because calyx does not make any guarantee of the order of execution for threads running in parallel, it is impossible for us to determine which thread will access r first for each iteration.

Nondeterminism when running parallel threads is beneficial on the compiler's end, as it will give the compiler more freedom for optimization. However, we sometimes do want to give parallel threads a measure of ordering while still taking advantage of the performance boost of parallelism. The `@sync` attribute allows us to do that.


## Using the `@sync` attribute

Now we want to modify the program above so that in every iteration, thread A always reads after thread B finishes incrementing using the `@sync` attribute with the following:

```
{{#include ../../examples/sync/sync-doc-example.futil:control}}
```

First and foremost, always remember to import "primitives/sync.futil" when using the @sync attribute.

The `@sync` syntax can only be marked with empty statements. `@sync` means that the thread
marked with a certain value, now called barrier index, for this attribute, must stop and wait for all other threads marked with the same barrier index to arrive, at which point they can proceed.

In the modified program above, we see that `incr_idx` and `incr_r` must both finish in order for either thread to go forth. Because `add_r_to_accm` is executed after `incr_idx` in thread A, we know that in each iteration, `incr_r` will always increment `r` before `add_r_to_accm` reads it. We've also inserted another barrier at the end of the while loops for each thread, which essentially means `add_r_to_accm` has to finish before either thread enters the next iteration.

## Synchronization in Branches
We can also have "barriers" in `if` branches:
```
{{#include ../../examples/sync/sync-if.futil:control}}
```
In this control program, both branches of thread 1 have statements marked with `@sync(1)`,
which syncs it up with thread 2.

Be really really careful when using the `@sync` attribute in conditional branches!
If the other thread sharing one "barrier" with your thread is blocked unconditionally,
then you would probably want to have the same `@sync` value in both branches; since
having the same `@sync` value in only one branch would likely lead to a "deadlock"
situation: if thread A is running in the unlocked branch while the thread B
has a "barrier" that is expecting two threads, thread B may never proceed because
thread A never arrives at the "barrier".

## More Complex Example
If you want to see a more complex design using `@sync`, see
[sync-dot-product](https://github.com/calyxir/calyx/blob/master/tests/correctness/sync/sync-dot-product.futil)

## Limitations

Currently we only support two threads sharing the same "barrier", i.e., only two threads can have control with the `@sync` attribute marked with the same value.


[par-undef]: ./undefined.md#semantics-of-par
 [m-struct]: http://composition.al/blog/2013/09/22/some-example-mvar-ivar-and-lvar-programs-in-haskell/
 [ex]: https://github.com/calyxir/calyx/blob/master/examples/sync/sync.futil