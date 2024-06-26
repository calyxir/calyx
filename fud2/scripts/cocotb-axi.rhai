import "calyx" as c;

export let cocotb_axi = state("cocotb-axi", ["dat"]);

export let cocotb_setup = cocotb_setup;
fn cocotb_setup(e) {
    e.config_var_or("cocotb-makefile-dir", "cocotb.makefile-dir", "$calyx-base/yxi/axi-calyx/cocotb");
    // TODO (nate): this is duplicated from the sim_setup above. Can this be shared?
    // The input data file. `sim.data` is required.
    let data_name = e.config_val("sim.data");
    let data_path = e.external_path(data_name);
    e.var_("sim_data", data_path);

    // Cocotb is wants files relative to the location of the makefile.
    // This is annoying to calculate on the fly, so we just copy necessary files to the build directory
    e.rule("copy", "cp $in $out");
    e.rule("make-cocotb", "make DATA_PATH=$sim_data VERILOG_SOURCE=$in COCOTB_LOG_LEVEL=CRITICAL > $out");
    // This cleans up the extra `make` cruft, leaving what is in between `{` and `}.`
    e.rule("cleanup-cocotb", "sed -n '/Output:/,/make\\[1\\]/{/Output:/d;/make\\[1\\]/d;p}' $in > $out");
}

op(
    "calyx-to-cocotb-axi",
    [c::calyx_setup, cocotb_setup],
    c::verilog_noverify,
    cocotb_axi,
    |e, input, output| {
        e.build_cmd(
            ["Makefile"],
            "copy",
            ["$cocotb-makefile-dir/Makefile"],
            [],
        );
        e.build_cmd(
            ["axi_test.py"],
            "copy",
            ["$cocotb-makefile-dir/axi_test.py"],
            [],
        );
        e.build_cmd(
            ["run_axi_test.py"],
            "copy",
            ["$cocotb-makefile-dir/run_axi_test.py"],
            [],
        );
        e.build_cmd(
            ["tmp.dat"],
            "make-cocotb",
            [input],
            ["Makefile", "axi_test.py", "run_axi_test.py"],
        );

        e.build_cmd([output], "cleanup-cocotb", ["tmp.dat"], []);
    }
)
