import "rtl_sim" as sim;

export const jq_state = state("jq", ["jq"]);

fn dat_to_jq_setup(e) {
    e.config_var_or("expr", "jq.expr", ".");
    e.config_var_or("jq", "jq.exe", "jq");
    e.config_var_or("flags", "jq.flags", "");
    e.var_("jqfile", e.external_path(e.config_or("jq.file", "")));

    e.rule(
        "jq-run",
        "$jq \"$expr\" $flags $in > $out"
    );

    e.rule(
        "jq-run-file",
        "$jq -f $jqfile $flags $in > $out"
    );
}

op(
    "dat-to-jq",
    [dat_to_jq_setup],
    sim::dat,
    jq_state,
    |e, input, output| {
        e.build_cmd([output], "jq-run", [input], []);
    }
);

op(
    "dat-to-jqfile",
    [dat_to_jq_setup],
    sim::dat,
    jq_state,
    |e, input, output| {
        e.build_cmd([output], "jq-run-file", [input], []);
    }
);