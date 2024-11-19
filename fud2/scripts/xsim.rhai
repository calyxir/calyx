import "rtl_sim" as sim;
import "testbench" as tb;
import "calyx" as c;

export const xsim_setup = xsim_setup;
fn xsim_setup(e) {
    e.config_var("vivado-dir", "xilinx.vivado");
    e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000");
    e.rsrc("xsim.tcl");
    let has_tcl = !e.config_or("xsim.ip_tcl", "").is_empty();
    if has_tcl {
        let tcl_name = e.config_val("xsim.ip_tcl");
        let tcl_path = e.external_path(tcl_name);
        e.var_("ip_tcl", tcl_path);
        print(tcl_path);
        e.rule(
           "xsim-standalone-tb",
           "$vivado-dir/bin/vivado -mode batch -source xsim.tcl -tclargs -c $cycle-limit -d $datadir -i $ip_tcl tb.sv verilog.sv > $out",
        );
        e.rule(
           "xsim-custom-tb",
           "$vivado-dir/bin/vivado -mode batch -source xsim.tcl -tclargs -c $cycle-limit -d $datadir -i $ip_tcl tb.sv verilog.sv memories.sv > $out",
        );
    } else {
        e.rule(
           "xsim-standalone-tb",
           "$vivado-dir/bin/vivado -mode batch -source xsim.tcl -tclargs -c $cycle-limit -d $datadir tb.sv verilog.sv > $out",
        );
        e.rule(
           "xsim-custom-tb",
           "$vivado-dir/bin/vivado -mode batch -source xsim.tcl -tclargs -c $cycle-limit -d $datadir tb.sv verilog.sv memories.sv > $out",
        );
    }
}

export const xsim_build = xsim_build;
fn xsim_build(e, input, output, datadir, standalone_tb) {
   if standalone_tb {
      e.build_cmd(
        [output],
        "xsim-standalone-tb",
        [input],
        ["tb.sv", "xsim.tcl", datadir],
      );
   } else {
      e.build_cmd(
         [output],
         "xsim-custom-tb",
         [input],
         ["tb.sv", "memories.sv", "xsim.tcl", datadir],
      );
   }
}

op(
   "xsim",
   [sim::sim_setup, tb::standalone_setup, xsim_setup],
   c::verilog_state,
   sim::dat,
   |e, input, output| { 
      xsim_build(e, input, "sim.log", "$datadir", true);
      e.build_cmd(
         [output],
         "json-data",
         ["$datadir", "sim.log"],
         ["json-dat.py"],
      );
   },
);

op(
   "xsim-refmem",
   [sim::sim_setup, tb::custom_setup, xsim_setup],
   tb::verilog_refmem,
   sim::dat,
   |e, input, output| {
      xsim_build(e, input, "sim.log", "$datadir", false);
      e.build_cmd(
         [output],
         "json-data",
         ["$datadir", "sim.log"],
         ["json-dat.py"],
      );
   }
);
