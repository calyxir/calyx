
import numpy as np
import tvm
from tvm import ir, relay
from aot import compile

def simple_example():
    # Declare a Relay module.
    mod = ir.IRModule()
    x = relay.var('x', shape=())
    f = relay.Function([x], x)
    
    # Compile the function.
    cfunc = compile(f, mod, tvm.context(1), tvm.target.arm_cpu())
    print (cfunc)

if __name__ == '__main__':
    simple_example()
