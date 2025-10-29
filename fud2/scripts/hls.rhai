export const cpp_state = state("cpp", ["cpp"]);

export const hls_report_state = state("hls-report", ["json"]);
export const hls_impl_report_state = state("hls-impl-report", ["json"]);

fn hls_setup(e) {
    e.config_var_or("vitis-hls-top", "vitis-hls.top", "kernel");

    e.rsrc("hls.tcl");

    e.rule("vitis-hls", "vitis_hls -f hls.tcl -tclargs top $vitis-hls-top > /dev/null");
    e.rule("vitis-hls-impl", "vitis_hls -f hls.tcl -tclargs top $vitis-hls-top impl > /dev/null");

    e.rule("extract-hls-json", "synthrep summary -m hls --top $vitis-hls-top > $out");
    e.rule("extract-hls-impl-json", "synthrep summary -m hls-impl --top $vitis-hls-top > $out");

    e.rule("copy", "cp $in $out");
}

fn hls_build(e, input, output, impl) {
    let suffix = if impl { "-impl" } else { "" };

    e.build_cmd(["kernel.cpp"], "copy", [input], []);
    e.build_cmd(["IGNOREME"], `vitis-hls${suffix}`, [], ["kernel.cpp", "hls.tcl"]);
    e.build_cmd([output], `extract-hls${suffix}-json`, ["IGNOREME"], []);
}

op(
    "vitis-hls",
    [hls_setup],
    cpp_state,
    hls_report_state,
    |e, input, output| {
        hls_build(e, input, output, false);
    },
);

op(
    "vitis-hls-impl",
    [hls_setup],
    cpp_state,
    hls_impl_report_state,
    |e, input, output| {
        hls_build(e, input, output, true);
    },
);
