import "rtl_sim" as sim;
import "testbench" as tb;
import "calyx" as c;

let dbg = state("debug", []);
let cider_state = state("cider", []);

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
       "run-cider",
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

op(
    "calyx-to-cider",
    [sim::sim_setup, c::calyx_setup],
    c::calyx_state,
    cider_state,
    |e, input, _output| {
        e.build_cmd(
            ["cider-input.futil"],
            "calyx-with-flags",
            [input],
            [],
        );
    },
);

op(
   "interp",
   [
      sim::sim_setup,
      tb::standalone_setup,
      c::calyx_setup,
      cider_setup,
    ],
    c::calyx_state,
    sim::dat,
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
    "cider",
    [sim::sim_setup, c::calyx_setup, cider_setup],
    cider_state,
    sim::dat,
    |e, _input, output| {
        let out_file = "interp_out.dump";
        e.build_cmd(
            [out_file],
            "run-cider",
            ["cider-input.futil"],
            ["data.dump"],
        );
        e.build_cmd(
            [output],
            "interp-to-dump",
            [out_file],
            ["$sim_data", "$cider-converter"],
        );
    },
);

op(
    "debug",
    [
        sim::sim_setup,
        tb::standalone_setup,
        c::calyx_setup,
        cider_setup,
    ],
    c::calyx_state,
    dbg,
    |e, input, output| {
        e.build_cmd([output], "cider-debug", [input], ["data.json"]);
    },
);
