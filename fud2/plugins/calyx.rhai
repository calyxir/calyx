fn calyx_setup(e) {
   e.config_var("calyx-base", "calyx.base");
   e.config_var_or("calyx-exe", "calyx.exe", "$calyx-base/target/debug/calyx");
   e.config_var_or("args", "calyx.args", "");
   e.rule("calyx", "$calyx-exe -l $calyx-base -b $backend $args $in > $out");
   e.rule("calyx-pass", "$calyx-exe -l $calyx-base -p $pass $args $in > $out");
}

op(
  "calyx2-to-verilog",
  [calyx_setup],
  get_state("calyx"),
  get_state("verilog"),
  |e, input, output| {
    e.build_cmd([output], "calyx", [input], []) ;
    e.arg("backend", "verilog");
  }
)
