# Run this by typing:
#
#   vivado -mode batch -source synth.tcl
#
# Then see the resource utilization (i.e., area) report dumped at:
#
#   out/FutilBuild.runs/synth_1/main_utilization_synth.rpt
#
# And if you also do implementation (see below), see the timing report:
#
#   out/FutilBuild.runs/impl_1/main_timing_summary_routed.rpt

# Settings: the output directory and the part number (which is a Zynq
# XC7Z020, found on our ZedBoard).
set outdir ./out
# set partname xc7z020clg484-1
# You can also use part name "xcu250-figd2104-2-e", which we get on havarti. 
# This is a bigger device (larger memory, etc.) and also supports URAM memory, which 
# "xczu3eg-sbva484-1-e" does not support. For more information on 
# this part type look here: https://docs.xilinx.com/r/en-US/ds962-u200-u250/Summary
set partname "xczu3eg-sbva484-1-e"

# Create the project (forcibly overwriting) and add sources SystemVerilog
# (*.sv) and Xilinx constraint files (*.xdc), which contain directives for
# connecting design signals to physical FPGA pins.
create_project -force -part $partname FutilBuild $outdir
add_files [glob ./*.sv]
add_files -fileset constrs_1 [glob ./*.xdc]
set_property top main [current_fileset]

# Switch the project to "out-of-context" mode, which frees us from the need to
# hook up every input & output wire to a physical device pin.
set_property \
    -name {STEPS.SYNTH_DESIGN.ARGS.MORE OPTIONS} \
    -value {-mode out_of_context -flatten_hierarchy "rebuilt"} \
    -objects [get_runs synth_1]

# Run synthesis. This is enough to generate the utilization report mentioned
# above but does not include timing information.
launch_runs synth_1
wait_on_run synth_1

# Run implementation to do place & route. This also produces the timing
# report mentioned above. Removing this step makes things go quite a bit
# faster if you just need the resource report!
launch_runs impl_1 -to_step route_design
wait_on_run impl_1
