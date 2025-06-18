# Dahlia

[Dahlia][] is an imperative, loop-based programming language for designing
hardware accelerators.

## Installation

First, install [sbt][] and [scala][].

Then, clone the repository and build the Dahlia compiler:
```
git clone https://github.com/cucapra/dahlia.git
cd dahlia
sbt compile
sbt assembly
chmod +x ./fuse
```

The Dahlia compiler can be run using the `./fuse` binary:
```
./fuse --help
```

Finally, configure `fud2` to use the Dahlia compiler by running `fud2 config` and adding the below to the file:
```
dahlia = "<path to Dahlia repository>/fuse"
```
<!-- Use `fud` to check if the compiler was installed correctly:
```
fud check
```
`fud` should report that the Dahlia compiler is available and has the right
version. -->

If something went wrong, try following the [instructions][] to build the Dahlia
compiler from its repository.

## Compiling Dahlia to Calyx

Dahlia programs can be compiled to Calyx using:
```
fud2 --from dahlia <input file> --to calyx
```

The Dahlia backed for Calyx is neither *complete* nor *stable*. If you find
a confusing error or wrong program, please open an [issue][].

[dahlia]: https://capra.cs.cornell.edu/dahlia
[instructions]: https://github.com/cucapra/dahlia#set-it-up
[issue]: https://github.com/cucapra/dahlia/issues
[sbt]: https://www.scala-sbt.org/1.x/docs/Setup.html
[scala]: https://docs.scala-lang.org/getting-started/index.html
