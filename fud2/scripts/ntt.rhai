import "calyx" as calyx;

export const ntt_state = state("ntt", ["ntt"]);

fn ntt_setup(e) {
   e.config_var("calyx-base", "calyx.base");
    e.config_var_or("python", "python", "python3");
    e.config_var_or("ntt-exe", "ntt.exe", "$calyx-base/frontends/ntt-pipeline/gen-ntt-pipeline.py");
    e.rule(
        "run-ntt",
        "$python $ntt-exe $in > $out"
    )
}

op(
    "ntt",
    [ntt_setup],
    ntt_state,
    calyx::calyx_state,
    |e, input, output| {
        e.build_cmd(
            [output],
            "run-ntt",
            [input],
            []
        )
    }
)
