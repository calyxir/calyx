//===- FuTILDialect.cpp - FuTIL dialect ---------------*- C++ -*-===//

#include "FuTIL/FuTILDialect.h"
#include "FuTIL/FuTILOps.h"

using namespace mlir;
using namespace mlir::futil;

//===----------------------------------------------------------------------===//
// FuTIL dialect.
//===----------------------------------------------------------------------===//

void FuTILDialect::initialize() {
  addOperations<
#define GET_OP_LIST
#include "FuTIL/FuTILOps.cpp.inc"
      >();
}
