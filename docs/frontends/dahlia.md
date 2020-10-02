# Dahlia

[Dahlia][] is an imperative, loop-based programming language for designing
hardware accelerators.

Follow the [instructions][] to build the Dahlia compiler from its repository.
Once the compiler is built, the `./fuse` binary in the Dahlia repository points
to the Dahlia compiler.

To compile a Dahlia program to FuTIL, run:
```
./fuse -b futil --lower <file>
```

This performs two steps:
- `--lower`: Compile away constructs such as unrolled loops and banked memories by rewriting Dahlia programs.
- `-b futil`: Generate the FuTIL program for the lowered program.

The Dahlia backed for FuTIL is neither *complete* nor *stable*. If you find
a confusing error or wrong program, please open an [issue][].

[dahlia]: https://capra.cs.cornell.edu/dahlia
[instructions]: https://github.com/cucapra/dahlia#set-it-up
[issue]: https://github.com/cucapra/dahlia/issues
