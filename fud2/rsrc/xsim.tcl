proc lshift listVar {
  upvar 1 $listVar L
  set r [lindex $L 0]
  set L [lreplace $L [set L 0] 0]
  return $r
}

#-------------------------------------------------------
# Process command line arguments
#-------------------------------------------------------
set error 0
set help 0
set cycle_limit 50000
set data ""
set verilog {}
set args $argv
# if {[llength $args] == 0} { incr help }; # Uncomment if necessary
while {[llength $args]} {
  set flag [lshift args]
  switch -exact -- $flag {
    -i -
    -ip-tcl {
      set ip_script [lshift args]
    }
    -d -
    -data {
      set data [lshift args]
    }
    -c -
    -cycle-limit {
      set cycle_limit [lshift args]
    }
    -h -
    -help {
      incr help
    }
    default {
      if {[string match "-*" $flag]} {
       puts " ERROR - option '$flag' is not a valid option."
       incr error
      } else {
        lappend verilog $flag
      }
    }
  }
}

if {$help} {
  set callerflag [lindex [info level [expr [info level] -1]] 0]
  # <-- HELP
  puts [format {
 Usage: %s
       [-ports|-p <listOfPorts>]
       [-verbose|-v]
       [-help|-h]

 Description: xxxxxxxxxxxxxxxxxxx.
        xxxxxxxxxxxxxxxxxxx.

 Example:
   %s -port xxxxxxxxxxxxxxx

  } $callerflag $callerflag ]
  # HELP -->
  return -code ok {}
}

# Check validity of arguments. Increment $error to generate an error

if {$error} {
  return -code error {Oops, something is not correct}
}

set dir [pwd]

create_project -force prj1
add_files $verilog
if {[info exists ip_script]} {
  source $ip_script
}
set_property top toplevel [get_fileset sim_1]
set_property -name {xsim.simulate.runtime} -value {all} -objects [get_filesets sim_1]
puts $cycle_limit
set_property verilog_define [subst {CYCLE_LIMIT=$cycle_limit DATA=$dir/$data}] [get_filesets sim_1]
launch_simulation
close_project
return -code ok {}
