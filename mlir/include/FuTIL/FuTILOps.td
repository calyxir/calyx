//===- FuTILOps.td - FuTIL dialect ops -------------------*- tablegen -*-===//

#ifndef FUTIL_OPS
#define FUTIL_OPS

include "FuTILDialect.td"
include "mlir/Interfaces/SideEffectInterfaces.td"

def FuTIL_FooOp : FuTIL_Op<"foo", [NoSideEffect,
                                             SameOperandsAndResultType]> {
    let summary = "Illustrates how to define an operation.";
    let description = [{
        The `futil.foo` operation illustrates how to define a new
        operation in a dialect. It uses an operation trait to declare that it
        has no side effects.

        This operation takes an integer argument and returns an integer.

        Example:

        ```mlir
        %0 = constant 2 : i32
        // Apply the foo operation to %0
        %1 = futil.foo %0 : i32
        ```
    }];

    let arguments = (ins I32:$input);
    let results = (outs I32:$res);

    let assemblyFormat = [{
        $input attr-dict `:` type($input)
    }];
}

#endif // FUTIL_OPS
