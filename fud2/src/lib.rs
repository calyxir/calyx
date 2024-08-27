use std::str::FromStr;

use fud_core::{
    exec::{SetupRef, StateRef},
    utils::basename,
    DriverBuilder,
};

fn setup_calyx(
    bld: &mut DriverBuilder,
    verilog: StateRef,
) -> (StateRef, SetupRef) {
    let calyx = bld.state("calyx", &["futil"]);
    let calyx_setup = bld.setup("Calyx compiler", |e| {
        e.config_var("calyx-base", "calyx.base")?;
        e.config_var_or(
            "calyx-exe",
            "calyx.exe",
            "$calyx-base/target/debug/calyx",
        )?;
        e.config_var_or("args", "calyx.args", "")?;
        e.rule(
            "calyx",
            "$calyx-exe -l $calyx-base -b $backend $args $in > $out",
        )?;
        e.rule(
            "calyx-pass",
            "$calyx-exe -l $calyx-base -p $pass $args $in > $out",
        )?;

        e.config_var_or("flags", "calyx.flags", "-p none")?;

        e.rule(
            "calyx-with-flags",
            "$calyx-exe -l $calyx-base $flags $args $in > $out",
        )?;

        Ok(())
    });
    bld.op(
        "calyx-to-verilog",
        &[calyx_setup],
        calyx,
        verilog,
        |e, input, output| {
            e.build_cmd(&[output[0]], "calyx", &[input[0]], &[])?;
            e.arg("backend", "verilog")?;
            Ok(())
        },
    );
    (calyx, calyx_setup)
}

pub fn build_driver(bld: &mut DriverBuilder) {
    // The verilog state
    let verilog = bld.state("verilog", &["sv", "v"]);
    let verilog_noverify = bld.state("verilog-noverify", &["sv", "v"]);

    // Get calyx setup
    let (_, calyx_setup) = setup_calyx(bld, verilog);

    let cocotb_setup = bld.setup("cocotb", |e| {
        e.config_var_or("cocotb-makefile-dir", "cocotb.makefile-dir", "$calyx-base/yxi/axi-calyx/cocotb")?;
        // TODO (nate): this is duplicated from the sim_setup above. Can this be shared?
        // The input data file. `sim.data` is required.
        let data_name = e.config_val("sim.data")?;
        let data_path = e.external_path(data_name.as_ref());
        e.var("sim_data", data_path.as_str())?;

        // Cocotb wants files relative to the location of the makefile.
        // This is annoying to calculate on the fly, so we just copy necessary files to the build directory
        e.rule("copy", "cp $in $out")?;

        let waves = e.config_constrained_or("waves", vec!["true", "false"], "false")?;
        let waves = FromStr::from_str(&waves).expect("The 'waves' flag should be either 'true' or 'false'.");
        if waves{
            //adds lines based on what is needed for icarus fst output.
            e.rule("iverilog-fst-sed",
            r#"sed '/\/\/ COMPONENT END: wrapper/c\`ifdef COCOTB_SIM\n  initial begin\n    \$$dumpfile ("$fst_file_name");\n    \$$dumpvars (0, wrapper);\n    #1;\n  end\n`endif\n\/\/ COMPONENT END: wrapper' $in > $out"#)?;
        }

e.var("cocotb-args", if waves {"WAVES=1"} else {""})?;

        e.rule("make-cocotb", "make DATA_PATH=$sim_data VERILOG_SOURCE=$in COCOTB_LOG_LEVEL=CRITICAL $cocotb-args > $out")?;
        // This cleans up the extra `make` and `FST warning` cruft, leaving what is in between `{` and `}.`
        e.rule("cleanup-cocotb", r#"sed -n '/Output:/,/make\[1\]/{/Output:/d;/make\[1\]/d;p}' $in | sed -n ':a;N;$$!ba;s/^[^{]*{\(.*\)}[^}]*$$/\1/p' | sed '1d;$$d' > $out"#)?;
        Ok(())
    });

    let cocotb_axi = bld.state("cocotb-axi", &["dat"]);
    // Example invocation: `fud2 <path to axi wrapped verilog> --from verilog-noverify --to cocotb-axi --set sim.data=<path to .data/json file>`
    bld.op(
        "calyx-to-cocotb-axi",
        &[calyx_setup, cocotb_setup],
        verilog_noverify,
        cocotb_axi,
        |e, input, output| {
            e.build_cmd(
                &["Makefile"],
                "copy",
                &["$cocotb-makefile-dir/Makefile"],
                &[],
            )?;
            e.build_cmd(
                &["axi_test.py"],
                "copy",
                &["$cocotb-makefile-dir/axi_test.py"],
                &[],
            )?;
            e.build_cmd(
                &["run_axi_test.py"],
                "copy",
                &["$cocotb-makefile-dir/run_axi_test.py"],
                &[],
            )?;
            let waves = e.config_constrained_or(
                "waves",
                vec!["true", "false"],
                "false",
            )?;
            let waves = FromStr::from_str(&waves)
                .expect("The 'waves' flag should be either 'true' or 'false'.");

            let vcd_file_name = format!("{}.fst", basename(input[0]));
            let mut make_in = input[0];
            if waves {
                make_in = "dumpvars.v";
                e.build_cmd(&[make_in], "iverilog-fst-sed", input, &[])?;
                e.arg("fst_file_name", &vcd_file_name)?;
            }
            e.build_cmd(
                &["tmp.dat"],
                "make-cocotb",
                &[make_in],
                &["Makefile", "axi_test.py", "run_axi_test.py"],
            )?;
            e.build_cmd(output, "cleanup-cocotb", &["tmp.dat"], &[])?;

            Ok(())
        },
    );
}
