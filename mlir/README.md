# MLIR Dialect for FuTIL

This is a dialect for FuTIL along with a standalone `opt`-like tool to operate on that dialect.

## Building and Setup

1. **Install Dependencies** of LLVM/MLIR according to [the
  instructions](https://mlir.llvm.org/getting_started/), including cmake and ninja.

2. **Check out and build [LLVM](https://github.com/llvm/llvm-project):**
```sh
git checkout https://github.com/llvm/llvm-project
cd llvm-project
mkdir build && cd build
cmake -G Ninja ../llvm -DLLVM_ENABLE_PROJECTS="mlir" -DLLVM_TARGETS_TO_BUILD="X86;RISCV" -DLLVM_ENABLE_ASSERTIONS=ON -DLLVM_INSTALL_UTILS=ON -DCMAKE_BUILD_TYPE=DEBUG
ninja
ninja check-mlir
```

3. **Build and test FuTIL dialect:**

```sh
cd futil/mlir
mkdir build && cd build
cmake -G Ninja .. -DMLIR_DIR=$PREFIX/lib/cmake/mlir -DLLVM_EXTERNAL_LIT=$BUILD_DIR/bin/llvm-lit
cmake --build . --target check-futil
```
To build the documentation from the TableGen description of the dialect operations, run
```sh
cmake --build . --target mlir-doc
```

## Running the test suite
```sh
cd futil/mlir/build
ninja check-futil
```