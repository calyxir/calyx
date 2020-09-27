// RUN: futil-opt %s | futil-opt | FileCheck %s

module {
    // CHECK-LABEL: func @bar()
    func @bar() {
        %0 = constant 1 : i32
        // CHECK: %{{.*}} = futil.foo %{{.*}} : i32
        %res = futil.foo %0 : i32
        return
    }
}
