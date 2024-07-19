export const verilog_state = state("verilog", ["sv", "v"]);
export const calyx_state = state("calyx", ["futil"]);
export const verilog_noverify = state("verilog-noverify", ["sv"]);

export let calyx_setup = calyx_setup;
export let calyx_pass = calyx_pass;

fn calyx_setup() {
    let calyx_base = config("calyx.base");
    #{ 
        calyx_base: calyx_base,
        calyx_exe: config_or("calyx.exe", `${calyx_base}/target/debug/calyx`),
        args: config_or("calyx.args", ""),
    }
}

defop calyx_to_verilog(c: calyx_state) >> v: verilog_state {
    let s = calyx_setup();
    shell(`${s.calyx_exe} -l ${s.calyx_base} -b verilog ${s.args} ${c} > ${v}`);
}

defop calyx_noverify(c: calyx_state) >> v: verilog_noverify {
    let s = calyx_setup();
    shell(`${s.calyx_exe} -l ${s.calyx_base} -b verilog ${s.args} --disable-verify ${c} > ${v}`);
}

fn calyx_pass(pass, args, in_str, out_str) {
    let s = calyx_setup();
    shell(`${s.calyx_exe} -l ${s.calyx_base} -p ${pass} ${args} ${in_str} > ${out_str}`);
}

fn calyx_setup(e) {
   e.config_var("calyx-base", "calyx.base");
   e.config_var_or("calyx-exe", "calyx.exe", "$calyx-base/target/debug/calyx");
   e.config_var_or("args", "calyx.args", "");
   e.rule("calyx", "$calyx-exe -l $calyx-base -b $backend $args $in > $out");
   e.rule("calyx-pass", "$calyx-exe -l $calyx-base -p $pass $args $in > $out");

   e.config_var_or("flags", "calyx.flags", "-p none");
   e.rule(
       "calyx-with-flags",
       "$calyx-exe -l $calyx-base $flags $args $in > $out",
   );
}

op(
  "calyx-to-verilog",
  [calyx_setup],
  calyx_state,
  verilog_state,
  |e, input, output| {
    e.build_cmd([output], "calyx", [input], []) ;
    e.arg("backend", "verilog");
  }
);


op(
    "calyx-noverify",
    [calyx_setup],
    calyx_state,
    verilog_noverify,
    |e, input, output| {
        // Icarus requires a special --disable-verify version of Calyx code.
        e.build_cmd([output], "calyx", [input], []);
        e.arg("backend", "verilog");
        e.arg("args", "--disable-verify");
    },
);
