# Makefile

# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog


#Needed to extract desired test from runt invocation

VERILOG_SOURCES += $(VERILOG_SOURCE)

#Defines build directory, if left to default only a single computation is run
SIM_BUILD=sim_build/

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = wrapper

# MODULE is the basename of the Python test file
MODULE = run_axi_test

# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim
