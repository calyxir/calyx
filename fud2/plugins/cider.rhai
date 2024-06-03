let dbg = state("debug2", []);

fn cider_setup(e) {
   e.config_var_or(
       "cider-exe",
       "cider.exe",
       "$calyx-base/target/debug/cider",
   );
   e.config_var_or(
       "cider-converter",
       "cider-converter.exe",
       "$calyx-base/target/debug/cider-data-converter",
   );
   e.rule(
       "cider",
       "$cider-exe -l $calyx-base --raw --data data.json $in > $out",
   );
   e.rule(
       "cider-debug",
       "$cider-exe -l $calyx-base --data data.json $in debug || true",
   );
   e.arg("pool", "console");

   // TODO Can we reduce the duplication around and `$python`?
   e.rsrc("interp-dat.py");
   e.config_var_or("python", "python", "python3");
   e.rule("dat-to-interp", "$python interp-dat.py --to-interp $in");
   e.rule(
       "interp-to-dat",
       "$python interp-dat.py --from-interp $in $sim_data > $out",
   );
   e.build_cmd(
       ["data.json"],
       "dat-to-interp",
       ["$sim_data"],
       ["interp-dat.py"],
   );

   e.rule(
       "cider2",
       "$cider-exe -l $calyx-base --data data.dump $in flat > $out",
   );

   e.rule("dump-to-interp", "$cider-converter --to cider $in > $out");
   e.rule("interp-to-dump", "$cider-converter --to json $in > $out");
   e.build_cmd(
       ["data.dump"],
       "dump-to-interp",
       ["$sim_data"],
       ["$cider-converter"],
   );
}

let sim_setup = get_setup("RTL simulation");
let tb_setup = get_setup("Standalone Testbench Setup");
let calyx_setup = get_setup("Calyx compiler");

op(
   "interp2",
   [
      sim_setup,
      tb_setup,
      calyx_setup,
      cider_setup,
    ],
    get_state("calyx"),
    get_state("dat"),
    |e, input, output| {
        let out_file = "interp_out.json";
        e.build_cmd([out_file], "cider", [input], ["data.json"]);
        e.build_cmd(
            [output],
            "interp-to-dat",
            [out_file],
            ["$sim_data", "interp-dat.py"],
        );
    },
);
op(
    "interp-flat2",
    [sim_setup, calyx_setup, cider_setup],
    get_state("calyx"),
    get_state("dat"),
    |e, input, output| {
        let out_file = "interp_out.dump";
        e.build_cmd([out_file], "cider2", [input], ["data.dump"]);
        e.build_cmd(
            [output],
            "interp-to-dump",
            [out_file],
            ["$sim_data", "$cider-converter"],
        );
    },
);
op(
    "debug2",
    [
        sim_setup,
        tb_setup,
        calyx_setup,
        cider_setup,
    ],
    get_state("calyx"),
    dbg,
    |e, input, output| {
        e.build_cmd([output], "cider-debug", [input], ["data.json"]);
    },
);
