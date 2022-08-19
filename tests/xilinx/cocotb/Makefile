# Makefile

# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog

VERILOG_SOURCES += $(TEST_PATH)/hdl/toplevel.v
VERILOG_SOURCES += $(TEST_PATH)/hdl/main.sv

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = Toplevel

# MODULE is the basename of the Python test file
MODULE = run_axi_test

# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim
