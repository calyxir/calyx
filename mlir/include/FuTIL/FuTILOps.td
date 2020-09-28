//===- FuTILOps.td - FuTIL dialect ops -------------------*- tablegen -*-===//

#ifndef FUTIL_OPS
#define FUTIL_OPS

include "FuTILDialect.td"
include "mlir/Interfaces/SideEffectInterfaces.td"

def FuTIL_AddOp : FuTIL_Op<"add", [Commutative, NoSideEffect,
                                             SameOperandsAndResultType]> {
    let summary = "Add operation.";
    let description = [{
        Takes in two integers and adds them together, returning an
        integer of the same type.
        Example:

        ```mlir
        %c0 = constant 1 : i32
        %c1 = constant 2 : i32
        %res = futil.add %c0, %c1 : (i32, i32) -> i32
        ```
    }];

    let arguments = (ins AnyType:$lhs, AnyType:$rhs);
    let results = (outs AnyType:$result);

    let assemblyFormat = [{
        $lhs `,` $rhs attr-dict `:`  `(` type($lhs) `,` type($rhs) `)`  `->` type($result)
    }];
}

#endif // FUTIL_OPS
