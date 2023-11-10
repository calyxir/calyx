# Usage: vivado_hls -f hls.tcl -tclargs [-impl] [-top <name>]

set impl 0
set top kernel
set hls_prj benchmark.prj

while {[llength $argv]} {
  switch -exact -- [lshift argv] {
    -impl {
      set impl 1
    }
    -top {
      set top [lshift argv]
    }
  }
}

open_project ${hls_prj} -reset
set_top $top; # The name of the hardware function.
add_files [glob ./*.cpp] -cflags "-std=c++11 -DVHLS"; # HLS source files.

open_solution "solution1"
set_part xczu3eg-sbva484-1-e
create_clock -period 7

# Actions we can take include csim_design, csynth_design, or cosim_design.
csynth_design

if {$impl} {
  export_design -format ip_catalog -version 1.1.0 -flow impl
}

exit
