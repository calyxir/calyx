//===- FuTILOps.h - FuTIL dialect ops ---------------------------*- C++ -*-===//

#ifndef FUTIL_FUTILOPS_H
#define FUTIL_FUTILOPS_H

#include "mlir/IR/Dialect.h"
#include "mlir/IR/OpDefinition.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

namespace mlir {
namespace futil {

#define GET_OP_CLASSES
#include "FuTIL/FuTILOps.h.inc"

} // namespace futil
} // namespace mlir

#endif // FUTIL_FUTILOPS_H
