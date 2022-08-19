import cocotb
import os


@cocotb.test()
async def run(toplevel):
    from axi_test import run_kernel_test
    print("hahaha")
    test_path = os.getcwd() + "/" + os.environ["TEST_PATH"]
    #TODO: use for file in listdir(test_path)
        #if file.endswith .data ...
    data_path = None
    expect_path = None
    for file in os.listdir(test_path):
        if file.endswith(".data"):
            data_path = test_path + "/" + file
        elif file.endswith("expect"):
            expect_path = test_path + "/" + file
    await run_kernel_test(toplevel, data_path, expect_path)

