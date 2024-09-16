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

    let waves = e.config_constrained_or("waves", ["true", "false"], "false");
    if waves == "true" {
        //adds lines based on what is needed for icarus fst output.
        e.rule("iverilog-fst-sed",
        `sed '/\/\/ COMPONENT END: wrapper/c\` + "ifdef COCOTB_SIM\n  initial begin\n" + `    \$$dumpfile ("$fst_file_name");` + "\n    " + `\$$dumpvars (0, wrapper);` + "\n    1;\n  end\n`endif\n" + `\/\/ COMPONENT END: wrapper' $in > $out`);
    }

    e.var_("cocotb-args", if waves == "true" {"WAVES=1"} else {""});
    e.rule("make-cocotb", "make DATA_PATH=$sim_data VERILOG_SOURCE=$in COCOTB_LOG_LEVEL=CRITICAL $cocotb-args > $out");

    // Cocotb is wants files relative to the location of the makefile.
    // This is annoying to calculate on the fly, so we just copy necessary files to the build directory
    e.rule("copy", "cp $in $out");
    // This cleans up the extra `make` cruft, leaving what is in between `{` and `}.`
    e.rule(
      "cleanup-cocotb", `sed -n '/Output:/,/make\[1\]/{/Output:/d;/make\[1\]/d;p}' $in > $out`
    );
}

fn basename(s) {
    let out = "";
    let dot_idx = s.len() - 1;
    while s[dot_idx] != "." && dot_idx >= 0 {
        dot_idx -= 1;
    }
    if dot_idx != 0 {
        s.truncate(dot_idx)
    } else {
        s
    }
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
        let waves = e.config_constrained_or(
            "waves",
            ["true", "false"],
            "false",
        );
        let vcd_file_name = `${basename(input)}.fst`;
        let make_in = input;
        if waves == "true" {
            make_in = "dumpvars.v";
            e.build_cmd([make_in], "iverilog-fst-sed", input, []);
            e.arg("fst_file_name", vcd_file_name);
        }
        e.build_cmd(
            ["tmp.dat"],
            "make-cocotb",
            [input],
            ["Makefile", "axi_test.py", "run_axi_test.py"],
        );

        e.build_cmd([output], "cleanup-cocotb", ["tmp.dat"], []);
    }
)
