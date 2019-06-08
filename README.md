# FuTIL
Fuse Temportal Intermediate Language  
An intermediate language for [Fuse](https://github.com/cucapra/seashell).

## Syntax
You can define new modules as follows:
```racket
(define/module name ((in1 : 32) (in2 : 32)) (out1 : 16)
  ...)
```

There are 3 kinds of statements that can go in the body:
- Module instantiation: `[name = new module]`
- Port connections: `[in1 -> out1]`
- Port splitting: `[name1 & name2 = split 16 in1]`

