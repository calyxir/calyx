# Makefile

# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog


#Needed to extract desired test from runt invocation

VERILOG_SOURCES += $(PWD)/../outputs/axi-combined.v

#Defines build directory, if left to default only a single computation is run
SIM_BUILD=sim_build/axi-combined

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = main

# MODULE is the basename of the Python test file
MODULE = axi-combined-tests


# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim
