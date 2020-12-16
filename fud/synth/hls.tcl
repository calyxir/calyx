set hls_prj benchmark.prj
open_project ${hls_prj} -reset
set_top kernel; # The name of the hardware function.
add_files [glob ./*.cpp] -cflags "-std=c++11 -DVHLS" ; # HLS source files.
open_solution "solution1"
set_part xczu3eg-sbva484-1-e
create_clock -period 7

# Actions we can take include csim_design, csynth_design, or cosim_design.
csynth_design

exit
