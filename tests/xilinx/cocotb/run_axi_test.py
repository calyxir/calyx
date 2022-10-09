import cocotb
import os


@cocotb.test()
async def run(toplevel):
    from axi_test import run_kernel_test

    test_path = os.getcwd() + "/" + os.path.basename(os.environ["TEST_PATH"])
    data_path = None
    for file in os.listdir(test_path):
        if file.endswith(".data"):
            data_path = test_path + "/" + file
    await run_kernel_test(toplevel, data_path)
