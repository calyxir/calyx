//===- futil-translate.cpp --------------------------------------*- C++ -*-===//

#include "mlir/InitAllTranslations.h"
#include "mlir/Support/LogicalResult.h"
#include "mlir/Translation.h"

#include "FuTIL/FuTILDialect.h"

int main(int argc, char **argv) {
  mlir::registerAllTranslations();

  // TODO: Register FuTIL translations here.

  return failed(
      mlir::mlirTranslateMain(argc, argv, "MLIR Translation Testing Tool"));
}
