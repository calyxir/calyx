# Makefile

# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog

#TODO (nathanielnrn): need to fix this so that MODULE can correcly find run_axi_test
# from directory Make is run from (or -C flag is used to)
# While also allowing the sources to be found correcly from our runt file
# Currently, this works if running from tests/xilinx/cocotb/ and TEST_PATH=vectorized-add
DIR_NAME := $(shell basename ${TEST_PATH})
VERILOG_SOURCES += $(PWD)/$(DIR_NAME)/hdl/toplevel.v
VERILOG_SOURCES += $(PWD)/$(DIR_NAME)/hdl/main.sv

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = Toplevel

# MODULE is the basename of the Python test file
MODULE = run_axi_test


# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim
