//===- FuTILOps.cpp - FuTIL dialect ops -------------------------*- C++ -*-===//

#include "FuTIL/FuTILOps.h"
#include "FuTIL/FuTILDialect.h"
#include "mlir/IR/OpImplementation.h"

namespace mlir {
namespace futil {
#define GET_OP_CLASSES
#include "FuTIL/FuTILOps.cpp.inc"

} // namespace futil
} // namespace mlir
