# MLIR Dialect for FuTIL

This is a dialect for FuTIL along with a standalone `opt`-like tool to operate on that dialect.

## Building and Setup

- **Install Dependencies** of LLVM/MLIR according to [the
  instructions](https://mlir.llvm.org/getting_started/), including cmake and ninja.

- **Check out and build [LLVM](https://github.com/llvm/llvm-project):**

```
$ git checkout https://github.com/llvm/llvm-project
$ cd llvm-project
$ mkdir build && cd build
$ cmake -G Ninja ../llvm -DLLVM_ENABLE_PROJECTS="mlir" -DLLVM_TARGETS_TO_BUILD="X86;RISCV" -DLLVM_ENABLE_ASSERTIONS=ON -DLLVM_INSTALL_UTILS=ON -DCMAKE_BUILD_TYPE=DEBUG
$ ninja
$ ninja check-mlir
```
- **Build and test FuTIL dialect:**
Assumes that you have built LLVM and MLIR in `$BUILD_DIR` and installed them to `$PREFIX`.
```
$ cd futil/mlir
$ mkdir build && cd build
$ cmake -G Ninja .. -DMLIR_DIR=$PREFIX/lib/cmake/mlir -DLLVM_EXTERNAL_LIT=$BUILD_DIR/bin/llvm-lit
$ cmake --build . --target check-futil
```
- (Optional) To build the documentation from the TableGen description of the dialect operations, run
```
$ cmake --build . --target mlir-doc
```

## Running the test suite
```
$ cd futil/mlir/build
$ ninja check-futil
```