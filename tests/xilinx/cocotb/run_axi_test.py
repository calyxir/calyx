import cocotb
import os
import logging


# TODO: Find out how to get rid of cocotb outputs
@cocotb.test()
async def run(toplevel):
    from axi_test import run_kernel_test

    toplevel._log.setLevel(logging.ERROR)
    test_path = os.getcwd() + "/" + os.path.basename(os.environ["TEST_PATH"])
    data_path = None
    expect_path = None
    for file in os.listdir(test_path):
        if file.endswith(".data"):
            data_path = test_path + "/" + file
        elif file.endswith("expect"):
            expect_path = test_path + "/" + file
    await run_kernel_test(toplevel, data_path, expect_path)
