import "rtl_sim" as sim;
import "testbench" as tb;
import "calyx" as c;

export const verilator_setup = verilator_setup;
fn verilator_setup(e) {
   e.config_var_or("verilator", "verilator.exe", "verilator");
   e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000");
   e.rule(
        "verilator-compile-standalone-tb",
        "$verilator $in tb.sv --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
   );
   e.rule(
        "verilator-compile-custom-tb",
        "$verilator $in tb.sv memories.sv --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
   );
   e.rule("cp", "cp $in $out");
}

export const verilator_build = verilator_build;
fn verilator_build(e, input, output, standalone_tb) {
   let out_dir = "verilator-out";
   let sim_bin = `${out_dir}/VTOP`;
   if standalone_tb {
      e.build_cmd(
        [sim_bin],
        "verilator-compile-standalone-tb",
        [input],
        ["tb.sv"],
      );
   } else {
      e.build_cmd(
         [sim_bin],
         "verilator-compile-custom-tb",
         [input],
         ["tb.sv", "memories.sv"],
      );
   }
   e.arg("out-dir", out_dir);
   e.build("cp", sim_bin, output);
}

op(
   "verilator",
   [sim::sim_setup, tb::standalone_setup, verilator_setup],
   c::verilog_state,
   sim::simulator,
   |e, input, output| { verilator_build(e, input, output, true) }
);

op(
   "verilator-refmem",
   [sim::sim_setup, tb::custom_setup, verilator_setup],
   tb::verilog_refmem,
   sim::simulator,
   |e, input, output| { verilator_build(e, input, output, false) }
);
