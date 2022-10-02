# Makefile

# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog


#Needed to extract desired test from runt invocation
ifdef TEST_PATH
DIR_NAME := $(shell basename ${TEST_PATH})
endif

VERILOG_SOURCES += $(PWD)/$(DIR_NAME)/hdl/toplevel.v
VERILOG_SOURCES += $(PWD)/$(DIR_NAME)/hdl/main.sv

#Defines build directory, if left to default only a single computation is run
SIM_BUILD=sim_build/$(DIR_NAME)

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = Toplevel

# MODULE is the basename of the Python test file
MODULE = run_axi_test


# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim
