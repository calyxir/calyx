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

fn verilator_build(e, input, output, standalone_tb) {
   let out_dir = "verilator-out";
   let sim_bin = `${out_dir}/VTOP`;
   if standalone_testbench {
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

let sim_setup = get_setup("RTL simulation");
let standalone_tb_setup = get_setup("Standalone Testbench Setup");

op(
   "verilator2",
   [sim_setup, standalone_tb_setup, verilator_setup],
   get_state("verilog"),
   get_state("sim"),
   |e, input, output| { verilator_build(e, input, output, true) }
);

op(
   "verilator-refmem2",
   [sim_setup, standalone_tb_setup, verilator_setup],
   get_state("verilog-refmem"),
   get_state("sim"),
   |e, input, output| { verilator_build(e, input, output, false) }
);
