//===- FuTILDialect.td - FuTIL dialect ---------------------*- tablegen -*-===//

#ifndef FUTIL_DIALECT
#define FUTIL_DIALECT

include "mlir/IR/OpBase.td"

//===----------------------------------------------------------------------===//
// FuTIL dialect definition.
//===----------------------------------------------------------------------===//

def FuTIL_Dialect : Dialect {
    let name = "futil";
    let summary = "A FuTIL out-of-tree MLIR dialect.";
    let description = [{
       This dialect defines the `futil` dialect, which is used to lower from
       Dahlia to Verilog.  For more information, see the
           [FuTIL GitHub page](https://github.com/cucapra/futil).
    }];
    let cppNamespace = "futil";
}

//===----------------------------------------------------------------------===//
// Base futil operation definition.
//===----------------------------------------------------------------------===//

class FuTIL_Op<string mnemonic, list<OpTrait> traits = []> :
        Op<FuTIL_Dialect, mnemonic, traits>;

#endif // FUTIL_DIALECT
