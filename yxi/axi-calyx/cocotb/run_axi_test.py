import cocotb
import os

#Idea is to have

#MAKE file -> calls cocotb, runs a single function from here
#this function looks for datapath

#Makefile sets datapath and verilog interested in testing

@cocotb.test()
async def run(toplevel):
    from axi_test import run_kernel_test

    data_path = os.environ.get("DATA_PATH")
    if not os.path.isabs(data_path):
        data_path = os.getcwd() + "/" + data_path
    assert data_path is not None and os.path.isfile(data_path), "DATA_PATH must be set and must be a valid file."

    await run_kernel_test(toplevel, data_path)




