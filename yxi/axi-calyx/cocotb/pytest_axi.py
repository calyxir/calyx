import os
from cocotb_tools.runner import get_runner, Verilog

LANGUAGE = os.getenv("HDL_TOPLEVEL_LANG", "verilog").lower().strip()


def test_axi_base():
    sim = os.getenv("SIM", "icarus")
    source = os.getenv("VERILOG_SOURCE")

    runner = get_runner(sim)
    runner.build(
        sources=[Verilog(source)],
        hdl_toplevel="Toplevel",
        verbose=True,
        always=True,
    )

    runner.test(hdl_toplevel="Toplevel", test_module="run_axi_test,")


if __name__ == "__main__":
    test_axi_base()
