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
    e.rule(
        "run-cider-debug",
        "$cider-exe -l $calyx-base --data data.dump $in debug || true",
    );
    e.arg("pool", "console");

    e.rule(
        "run-cider",
        "$cider-exe -l $calyx-base --data data.dump $in > $out",
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
    |e, input, output| {
        e.build_cmd(
            [output],
            "calyx-with-flags",
            [input],
            [],
        );
    },
);


op(
    "cider",
    [sim::sim_setup, c::calyx_setup, cider_setup],
    cider_state,
    sim::dat,
    |e, input, output| {
        let out_file = "interp_out.dump";
        e.build_cmd(
            [out_file],
            "run-cider",
            [input],
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
    cider_state,
    dbg,
    |e, input, output| {
        e.build_cmd([output], "run-cider-debug", [input], ["data.dump"]);
    },
);
