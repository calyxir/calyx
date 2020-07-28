# The list of pins on the Zynq is here:
#
# https://www.xilinx.com/support/packagefiles/z7packages/xc7z020clg484pkg.txt
#
# I don't have the first clue what the difference between the pins is, if any,
# so my main goal here is just to use pins that actually exist. Y9 also seems
# to be the pin that the clock is connected to on the ZedBoard, so I'll use
# that??

# Clock port.
set_property PACKAGE_PIN Y9 [get_ports clk]
set_property IOSTANDARD LVCMOS18 [get_ports clk]

# Connect the clock. This is where we determine the target frequency for the
# design, which of course is super important! Just sticking with 50 ns, which
# translates to 20 MHz (which is very slow!!).
create_clock -period 50.000 -name clk -waveform {0.000 25.000} [get_ports clk]

# Go port.
set_property PACKAGE_PIN F21 [get_ports go]
set_property IOSTANDARD LVCMOS18 [get_ports go]

# Done port.
set_property PACKAGE_PIN F22 [get_ports done]
set_property IOSTANDARD LVCMOS18 [get_ports done]
