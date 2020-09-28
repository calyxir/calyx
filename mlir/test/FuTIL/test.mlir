// RUN: futil-opt %s | futil-opt | FileCheck %s

module {
    // CHECK-LABEL: func @and_op()
    func @and_op() {
        %c0 = constant 1 : i32
        %c1 = constant 2 : i32
        // CHECK: %{{.*}} = futil.add %{{.*}}, %{{.*}} : (i32, i32) -> i32
        %res = futil.add %c0, %c1 : (i32, i32) -> i32
        return
    }
}
