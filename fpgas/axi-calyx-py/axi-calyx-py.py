### This file is intended to be a manually written of a python program
### That takes in that assumes a toplevel component with a single input memory
### And a single output memory. These memories are defined in this "axi_wrapper" component
### and the toplevel component accesses these memories through ref, which we assume the toplevel
### already does.

### It may also make sense for each memory to have its own "axi_control" unit, which in that case would have the actual instantiation of the memory,
### and be invoked/passed by reference into the higher level wrapper.

### Our imaginary main sums up the elements in a
### vector of size 16 an puts in output_mem[0] and puts the product of elements
### In output_mem[1]

#See https://github.com/cucapra/calyx/issues/1733#issuecomment-1765043603 for a good explanation of what we are trying to do.


from calyx.builder import Builder

def add_axi_wrapper_component(prog):
    axi_wrapper = prog.component("axi_wrapper")
    main.input()#TODO(nathanielnrn): What inputs do we need?)

    #TODO(nathanielnrn): Should we use std memorioes or seq memories? See #
    #TODO(nathanielnrn): For now memories will live in in this axi_wrapper, probably want to move to axi_controller_wrapper at some point

    #Input mem parameters need to be taken from .yxi file
    main_input_mem = axi_wrapper.mem_d1("input_mem",32, 16, 5)
    main_output_mem = axi_wrappermem_d1("output-mem", 32, 2, 1)



    this = axi_wrapper.this()


