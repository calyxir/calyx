import "rtl_sim" as sim;
import "testbench" as tb;
import "calyx" as c;

let dbg = state("cider-debug", []);
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
    e.config_var_or("converter-flags", "cider.converter-flags", "");
    e.config_var_or("cider-flags", "cider.flags", "");

    let has_data = !e.config_or("sim.data", "").is_empty();

    if has_data {
        e.var_("data", "--data data.dump");

        // copied from rtl_sim.rhai, we only want to do this when the sim.data
        // flag is present
        let data_name = e.config_val("sim.data");
        let data_path = e.external_path(data_name);
        e.var_("sim_data", data_path);
    } else {
        e.var_("data", "")
    }

    e.rule(
        "run-cider-debug",
    //                                                       this is a dumb hack
    //                                                       to avoid fud2
    //                                                       complaining about
    //                                                       there being no file
        "$cider-exe -l $calyx-base $data $cider-flags $in debug  && echo > $out",
    );
    e.arg("pool", "console");

    e.rule(
        "run-cider",
        "$cider-exe -l $calyx-base $data $cider-flags $in > $out",
    );

    e.rule("interp-to-dump", "$cider-converter --to json $converter-flags $in > $out");

    if has_data {
        e.rule("dump-to-interp", "$cider-converter --to cider $converter-flags $in > $out");
        e.build_cmd(
            ["data.dump"],
            "dump-to-interp",
            ["$sim_data"],
            ["$cider-converter"],
        );
    }
}

op(
    "calyx-to-cider",
    [c::calyx_setup],
    c::calyx_state,
    cider_state,
    |e, input, output| {
        e.build_cmd(
            [output],
            "calyx-cider",
            [input],
            [],
        );
    },
);


op(
    "cider",
    [c::calyx_setup, cider_setup],
    cider_state,
    sim::dat,
    |e, input, output| {
        let out_file = "interp_out.dump";
        let dependencies = if e.config_or("sim.data", "").is_empty() { [] } else { ["data.dump"] };
        e.build_cmd(
            [out_file],
            "run-cider",
            [input],
            dependencies,
        );
        e.build_cmd(
            [output],
            "interp-to-dump",
            [out_file],
            ["$cider-converter"],
        );
    },
);

op(
    "debug",
    [
        tb::standalone_setup,
        c::calyx_setup,
        cider_setup,
    ],
    cider_state,
    dbg,
    |e, input, output| {
        let dependencies = if e.config_or("sim.data", "").is_empty() { [] } else { ["data.dump"] };
        e.build_cmd([output], "run-cider-debug", [input], dependencies);
    },
);
