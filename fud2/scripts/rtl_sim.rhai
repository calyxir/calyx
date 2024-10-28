// This file provides fud2 operations for simulating Calyx

export const dat = state("dat", ["json"]);
export const vcd = state("vcd", ["vcd"]);
export const simulator = state("sim", ["exe"]);

export const sim_setup = sim_setup;

fn sim_setup(e) {
    // Data conversion to and from JSON.
    e.config_var_or("python", "python", "python3");
    e.rsrc("json-dat.py");
    e.rule("hex-data", "$python json-dat.py --from-json $in $out");
    e.rule("json-data", "$python json-dat.py --to-json $out $in");

    // The input data file. `sim.data` is required.
    let data_name = e.config_val("sim.data");
    let data_path = e.external_path(data_name);
    e.var_("sim_data", data_path);

    // Produce the data directory.
    e.var_("datadir", "sim_data");
    e.build_cmd(
        ["$datadir"],
        "hex-data",
        ["$sim_data"],
        ["json-dat.py"],
    );

    // Rule for simulation execution.
    e.rule(
        "sim-run",
        "./$bin +DATA=$datadir +CYCLE_LIMIT=$cycle-limit $args > $out",
    );

    // More shared configuration.
    e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000");
}

op(
    "simulate",
    [sim_setup],
    simulator,
    dat,
    |e, input, output| {
        e.build_cmd(["sim.log"], "sim-run", [input, "$datadir"], []);
        e.arg("bin", input);
        e.arg("args", "+NOTRACE=1");
        e.build_cmd(
            [output],
            "json-data",
            ["$datadir", "sim.log"],
            ["json-dat.py"],
        );
    },
);

op("trace", [sim_setup], simulator, vcd, |e, input, output| {
    e.build_cmd(
        ["sim.log", output],
        "sim-run",
        [input, "$datadir"],
        [],
    );
    e.arg("bin", input);
    e.arg("args", `+NOTRACE=0 +OUT=${output}`);
});
