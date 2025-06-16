import "calyx" as c;

export const report_state = state("report", ["rpt"]);

export const synth_setup = synth_setup;

fn synth_setup(e) {
  e.config_var("calyx-base", "calyx.base");
  e.rule("mkdir", "mkdir -p $out");
  e.rule("move", "mv $in $out");
  e.rule("copyspcl", "cp out/hierarchical_utilization_placed.rpt $out");
  e.rule("vivado", "vivado -mode batch -source synth.tcl");
  e.rsrc("synth.tcl");
  e.rsrc("device.xdc");
}

op(
  "verilog-to-report",
  [synth_setup, c::calyx_setup],
  c::calyx_state,
  report_state,
  |e, input, output| {
    e.build_cmd(["main.sv"], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", "--synthesis -p external --disable-verify");
    e.build_cmd(["IGNOREME.txt"], "vivado", [], ["main.sv", "synth.tcl", "device.xdc"]);
    e.build_cmd([output], "copyspcl", ["IGNOREME.txt"], []);
  }
);
