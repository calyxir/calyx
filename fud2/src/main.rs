use fud_core::{
    cli,
    exec::{SetupRef, StateRef},
    run::{EmitResult, Emitter},
    Driver, DriverBuilder,
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
        e.rule(
            "calyx",
            "$calyx-exe -l $calyx-base -b $backend $args $in > $out",
        )?;
        Ok(())
    });
    bld.op(
        "calyx-to-verilog",
        &[calyx_setup],
        calyx,
        verilog,
        |e, input, output| {
            e.build_cmd(&[output], "calyx", &[input], &[])?;
            e.arg("backend", "verilog")?;
            Ok(())
        },
    );
    (calyx, calyx_setup)
}

fn setup_dahlia(
    bld: &mut DriverBuilder,
    calyx: StateRef,
) -> (StateRef, SetupRef) {
    let dahlia = bld.state("dahlia", &["fuse"]);
    let dahlia_setup = bld.setup("Dahlia compiler", |e| {
        e.config_var("dahlia-exe", "dahlia")?;
        e.rule(
            "dahlia-to-calyx",
            "$dahlia-exe -b calyx --lower -l error $in -o $out",
        )?;
        Ok(())
    });
    bld.rule(&[dahlia_setup], dahlia, calyx, "dahlia-to-calyx");
    (dahlia, dahlia_setup)
}

fn setup_mrxl(
    bld: &mut DriverBuilder,
    calyx: StateRef,
) -> (StateRef, SetupRef) {
    let mrxl = bld.state("mrxl", &["mrxl"]);
    let mrxl_setup = bld.setup("MrXL compiler", |e| {
        e.var("mrxl-exe", "mrxl")?;
        e.rule("mrxl-to-calyx", "$mrxl-exe $in > $out")?;
        Ok(())
    });
    bld.rule(&[mrxl_setup], mrxl, calyx, "mrxl-to-calyx");
    (mrxl, mrxl_setup)
}

fn build_driver() -> Driver {
    let mut bld = DriverBuilder::new("fud2");

    // The verilog state
    let verilog = bld.state("verilog", &["sv", "v"]);
    // Calyx.
    let (calyx, calyx_setup) = setup_calyx(&mut bld, verilog);
    // Dahlia.
    setup_dahlia(&mut bld, calyx);
    // MrXL.
    setup_mrxl(&mut bld, calyx);

    // Shared machinery for RTL simulators.
    let dat = bld.state("dat", &["json"]);
    let vcd = bld.state("vcd", &["vcd"]);
    let simulator = bld.state("sim", &["exe"]);
    let sim_setup = bld.setup("RTL simulation", |e| {
        // Data conversion to and from JSON.
        e.config_var_or("python", "python", "python3")?;
        e.var(
            "json_dat",
            &format!("$python {}/json-dat.py", e.config_val("rsrc")?),
        )?;
        e.rule("hex-data", "$json_dat --from-json $in $out")?;
        e.rule("json-data", "$json_dat --to-json $out $in")?;

        // The input data file. `sim.data` is required.
        let data_name = e.config_val("sim.data")?;
        let data_path = e.external_path(data_name.as_ref());
        e.var("sim_data", data_path.as_str())?;

        // Produce the data directory.
        e.var("datadir", "sim_data")?;
        e.build("hex-data", "$sim_data", "$datadir")?;

        // Rule for simulation execution.
        e.rule(
            "sim-run",
            "./$bin +DATA=$datadir +CYCLE_LIMIT=$cycle-limit $args > $out",
        )?;

        // More shared configuration.
        e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000")?;

        Ok(())
    });
    // If we pass through firrtl-with-primitives, it will set its own custom refmem testbench.
    let testbench_normal_setup = bld.setup("Normal Testbench Setup", |e| {
        // The Verilog testbench.
        e.var("testbench", &format!("{}/tb.sv", e.config_val("rsrc")?))?;
        Ok(())
    });
    bld.op(
        "simulate",
        &[sim_setup],
        simulator,
        dat,
        |e, input, output| {
            e.build_cmd(&["sim.log"], "sim-run", &[input, "$datadir"], &[])?;
            e.arg("bin", input)?;
            e.arg("args", "+NOTRACE=1")?;
            e.build_cmd(&[output], "json-data", &["$datadir", "sim.log"], &[])?;
            Ok(())
        },
    );
    bld.op("trace", &[sim_setup], simulator, vcd, |e, input, output| {
        e.build_cmd(
            &["sim.log", output],
            "sim-run",
            &[input, "$datadir"],
            &[],
        )?;
        e.arg("bin", input)?;
        e.arg("args", &format!("+NOTRACE=0 +OUT={}", output))?;
        Ok(())
    });

    // Icarus Verilog.
    let verilog_noverify = bld.state("verilog-noverify", &["sv"]);
    let verilog_refmem = bld.state("verilog-refmem", &["sv"]);
    let verilog_noverify_refmem = bld.state("verilog-noverify-refmem", &["sv"]); // Need to use alternative testbench.
    let icarus_setup = bld.setup("Icarus Verilog", |e| {
        e.var("iverilog", "iverilog")?;
        e.rule(
            "icarus-compile",
            "$iverilog -g2012 -o $out $testbench $additional_input $in",
        )?;
        Ok(())
    });
    bld.op(
        "calyx-noverify",
        &[calyx_setup],
        calyx,
        verilog_noverify,
        |e, input, output| {
            // Icarus requires a special --disable-verify version of Calyx code.
            e.build_cmd(&[output], "calyx", &[input], &[])?;
            e.arg("backend", "verilog")?;
            e.arg("args", "--disable-verify")?;
            Ok(())
        },
    );
    bld.op(
        "icarus",
        &[sim_setup, testbench_normal_setup, icarus_setup],
        verilog_noverify,
        simulator,
        |e, input, output| {
            e.build("icarus-compile", input, output)?;
            Ok(())
        },
    );
    bld.op(
        "icarus-refmem",
        &[sim_setup, icarus_setup],
        verilog_noverify_refmem,
        simulator,
        |e, input, output| {
            e.build("icarus-compile", input, output)?;
            Ok(())
        },
    );

    // Calyx to FIRRTL.
    let firrtl = bld.state("firrtl", &["fir"]);
    let firrtl_with_primitives = bld.state("firrtl-with-primitives", &["fir"]);
    bld.op(
        "calyx-to-firrtl",
        &[calyx_setup],
        calyx,
        firrtl,
        |e, input, output| {
            e.build_cmd(&[output], "calyx", &[input], &[])?;
            e.arg("backend", "firrtl")?;
            e.arg("args", "--emit-primitive-extmodules")?;
            Ok(())
        },
    );

    let firrtl_primitives_setup = bld.setup("FIRRTL with primitives", |e| {
        // Convert all @external cells to ref cells.
        e.rule("external-to-ref", "sed 's/@external([0-9]*)/ref/g' $in | sed 's/@external/ref/g' > $out")?;

        // Produce FIRRTL with FIRRTL-defined primitives.
        e.var(
            "gen-firrtl-primitives-script",
            "$calyx-base/tools/firrtl/generate-firrtl-with-primitives.py",
        )?;
        e.rule(
            "generate-firrtl-with-primitives",
            "python3 $gen-firrtl-primitives-script $in > $out",
        )?;

        // Produce a custom testbench that handles memory reading and writing.
        e.var("testbench", "refmem_tb.sv")?;
        e.var(
            "gen-testbench-script",
            "$calyx-base/tools/firrtl/generate-testbench.py"
        )?;
        e.var("additional_input", &format!("{}/comb_mem_d1.sv", e.config_val("rsrc")?))?;

        e.rule("generate-refmem-testbench",
        "python3 $gen-testbench-script $in | tee $testbench $out"
        )?;

        e.rule("dummy", "cat $in $out")?;

        Ok(())
    });

    // Generates FIRRTL with FIRRTL definition of primitives
    bld.op(
        "firrtl-with-primitives",
        &[calyx_setup, firrtl_primitives_setup],
        calyx,
        firrtl_with_primitives,
        |e, input, output| {
            let tmp_calyx = "partial.futil";
            let tmp_firrtl = "partial.fir";
            let tmp_json = "primitive-uses.json";
            let dummy_testbench = "refmem-tb-copy.sv";
            // replace extmodule with ref
            e.build_cmd(&[tmp_calyx], "external-to-ref", &[input], &[])?;
            // generate the testbench
            e.build_cmd(
                &[dummy_testbench],
                "generate-refmem-testbench",
                &[tmp_calyx],
                &[],
            )?;
            // get original firrtl
            e.build_cmd(&[tmp_firrtl], "calyx", &[tmp_calyx], &[])?;
            e.arg("backend", "firrtl")?;
            e.arg("args", "--synthesis")?;
            // get primitive uses json
            e.build_cmd(&[tmp_json], "calyx", &[tmp_calyx], &[])?;
            e.arg("backend", "primitive-uses")?;
            e.arg("args", "--synthesis")?;
            // output whole FIRRTL program
            e.build_cmd(
                &[output],
                "generate-firrtl-with-primitives",
                &[tmp_firrtl, tmp_json, dummy_testbench],
                &[],
            )?;
            Ok(())
        },
    );

    // The FIRRTL compiler.
    let firrtl_setup = bld.setup("Firrtl to Verilog compiler", |e| {
        e.config_var("firrtl-exe", "firrtl.exe")?;
        e.rule("firrtl", "$firrtl-exe -i $in -o $out -X sverilog")?;

        e.var(
            "primitives-for-firrtl",
            &format!("{}/primitives-for-firrtl.sv", e.config_val("rsrc")?),
        )?;
        e.rule("add-firrtl-prims", "cat $primitives-for-firrtl $in > $out")?;

        Ok(())
    });
    fn firrtl_compile(
        e: &mut Emitter,
        input: &str,
        output: &str,
    ) -> EmitResult {
        let tmp_verilog = "partial.sv";
        e.build_cmd(&[tmp_verilog], "firrtl", &[input], &[])?;
        e.build_cmd(&[output], "add-firrtl-prims", &[tmp_verilog], &[])?;
        Ok(())
    }
    fn firrtl_with_primitives_compile(
        e: &mut Emitter,
        input: &str,
        output: &str,
    ) -> EmitResult {
        e.build_cmd(&[output], "firrtl", &[input], &[])?;
        Ok(())
    }
    bld.op("firrtl", &[firrtl_setup], firrtl, verilog, firrtl_compile);
    bld.op(
        "firrtl-with-primitives-compile",
        &[firrtl_setup],
        firrtl_with_primitives,
        verilog_refmem,
        firrtl_with_primitives_compile,
    );
    // This is a bit of a hack, but the Icarus-friendly "noverify" state is identical for this path
    // (since FIRRTL compilation doesn't come with verification).
    bld.op(
        "firrtl-noverify",
        &[firrtl_setup],
        firrtl,
        verilog_noverify,
        firrtl_compile,
    );
    bld.op(
        "firrtl-with-primitives-noverify",
        &[firrtl_setup],
        firrtl_with_primitives,
        verilog_noverify_refmem,
        firrtl_with_primitives_compile,
    );

    // primitive-uses backend
    let primitive_uses_json = bld.state("primitive-uses-json", &["json"]);
    bld.op(
        "primitive-uses",
        &[calyx_setup],
        calyx,
        primitive_uses_json,
        |e, input, output| {
            e.build_cmd(&[output], "calyx", &[input], &[])?;
            e.arg("backend", "primitive-uses")?;
            Ok(())
        },
    );

    // Verilator.
    let verilator_setup = bld.setup("Verilator", |e| {
        e.config_var_or("verilator", "verilator.exe", "verilator")?;
        e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000")?;
        e.rule(
            "verilator-compile",
            "$verilator $in $testbench $additional_input --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
        )?;
        e.rule("cp", "cp $in $out")?;
        Ok(())
    });
    fn verilator_build(
        e: &mut Emitter,
        input: &str,
        output: &str,
    ) -> EmitResult {
        let out_dir = "verilator-out";
        let sim_bin = format!("{}/VTOP", out_dir);
        e.build("verilator-compile", input, &sim_bin)?;
        e.arg("out-dir", out_dir)?;
        e.build("cp", &sim_bin, output)?;
        Ok(())
    }
    bld.op(
        "verilator",
        &[sim_setup, testbench_normal_setup, verilator_setup],
        verilog,
        simulator,
        verilator_build,
    );
    bld.op(
        "verilator-refmem",
        &[sim_setup, verilator_setup],
        verilog_refmem,
        simulator,
        verilator_build,
    );

    // Interpreter.
    let debug = bld.state("debug", &[]); // A pseudo-state.
    let cider_setup = bld.setup("Cider interpreter", |e| {
        e.config_var_or(
            "cider-exe",
            "cider.exe",
            "$calyx-base/target/debug/cider",
        )?;
        e.rule(
            "cider",
            "$cider-exe -l $calyx-base --raw --data data.json $in > $out",
        )?;
        e.rule(
            "cider-debug",
            "$cider-exe -l $calyx-base --data data.json $in debug || true",
        )?;
        e.arg("pool", "console")?;

        // TODO Can we reduce the duplication around `rsrc_dir` and `$python`?
        let rsrc_dir = e.config_val("rsrc")?;
        e.var("interp-dat", &format!("{}/interp-dat.py", rsrc_dir))?;
        e.config_var_or("python", "python", "python3")?;
        e.rule("dat-to-interp", "$python $interp-dat --to-interp $in")?;
        e.rule(
            "interp-to-dat",
            "$python $interp-dat --from-interp $in $sim_data > $out",
        )?;
        e.build_cmd(&["data.json"], "dat-to-interp", &["$sim_data"], &[])?;
        Ok(())
    });
    bld.op(
        "interp",
        &[sim_setup, testbench_normal_setup, calyx_setup, cider_setup],
        calyx,
        dat,
        |e, input, output| {
            let out_file = "interp_out.json";
            e.build_cmd(&[out_file], "cider", &[input], &["data.json"])?;
            e.build_cmd(
                &[output],
                "interp-to-dat",
                &[out_file],
                &["$sim_data"],
            )?;
            Ok(())
        },
    );
    bld.op(
        "debug",
        &[sim_setup, testbench_normal_setup, calyx_setup, cider_setup],
        calyx,
        debug,
        |e, input, output| {
            e.build_cmd(&[output], "cider-debug", &[input], &["data.json"])?;
            Ok(())
        },
    );

    // Xilinx compilation.
    let xo = bld.state("xo", &["xo"]);
    let xclbin = bld.state("xclbin", &["xclbin"]);
    let xilinx_setup = bld.setup("Xilinx tools", |e| {
        // Locations for Vivado and Vitis installations.
        e.config_var("vivado-dir", "xilinx.vivado")?;
        e.config_var("vitis-dir", "xilinx.vitis")?;

        // Package a Verilog program as an `.xo` file.
        let rsrc_dir = e.config_val("rsrc")?;
        e.var("gen-xo-tcl", &format!("{}/gen_xo.tcl", rsrc_dir))?;
        e.var("get-ports", &format!("{}/get-ports.py", rsrc_dir))?;
        e.config_var_or("python", "python", "python3")?;
        e.rule("gen-xo", "$vivado-dir/bin/vivado -mode batch -source $gen-xo-tcl -tclargs $out `$python $get-ports kernel.xml`")?;
        e.arg("pool", "console")?;  // Lets Ninja stream the tool output "live."

        // Compile an `.xo` file to an `.xclbin` file, which is where the actual EDA work occurs.
        e.config_var_or("xilinx-mode", "xilinx.mode", "hw_emu")?;
        e.config_var_or("platform", "xilinx.device", "xilinx_u50_gen3x16_xdma_201920_3")?;
        e.rule("compile-xclbin", "$vitis-dir/bin/v++ -g -t $xilinx-mode --platform $platform --save-temps --profile.data all:all:all --profile.exec all:all:all -lo $out $in")?;
        e.arg("pool", "console")?;

        Ok(())
    });
    bld.op(
        "xo",
        &[calyx_setup, xilinx_setup],
        calyx,
        xo,
        |e, input, output| {
            // Emit the Verilog itself in "synthesis mode."
            e.build_cmd(&["main.sv"], "calyx", &[input], &[])?;
            e.arg("backend", "verilog")?;
            e.arg("args", "--synthesis -p external")?;

            // Extra ingredients for the `.xo` package.
            e.build_cmd(&["toplevel.v"], "calyx", &[input], &[])?;
            e.arg("backend", "xilinx")?;
            e.build_cmd(&["kernel.xml"], "calyx", &[input], &[])?;
            e.arg("backend", "xilinx-xml")?;

            // Package the `.xo`.
            e.build_cmd(
                &[output],
                "gen-xo",
                &[],
                &["main.sv", "toplevel.v", "kernel.xml"],
            )?;
            Ok(())
        },
    );
    bld.op("xclbin", &[xilinx_setup], xo, xclbin, |e, input, output| {
        e.build_cmd(&[output], "compile-xclbin", &[input], &[])?;
        Ok(())
    });

    // Xilinx execution.
    // TODO Only does `hw_emu` for now...
    let xrt_setup = bld.setup("Xilinx execution via XRT", |e| {
        // Generate `emconfig.json`.
        e.rule("emconfig", "$vitis-dir/bin/emconfigutil --platform $platform")?;
        e.build_cmd(&["emconfig.json"], "emconfig", &[], &[])?;

        // Execute via the `xclrun` tool.
        e.config_var("xrt-dir", "xilinx.xrt")?;
        e.rule("xclrun", "bash -c 'source $vitis-dir/settings64.sh ; source $xrt-dir/setup.sh ; XRT_INI_PATH=$xrt_ini EMCONFIG_PATH=. XCL_EMULATION_MODE=$xilinx-mode $python -m fud.xclrun --out $out $in'")?;
        e.arg("pool", "console")?;

        // "Pre-sim" and "post-sim" scripts for simulation.
        e.rule("echo", "echo $contents > $out")?;
        e.build_cmd(&["pre_sim.tcl"], "echo", &[""], &[""])?;
        e.arg("contents", "open_vcd\\\\nlog_vcd *\\\\n")?;
        e.build_cmd(&["post_sim.tcl"], "echo", &[""], &[""])?;
        e.arg("contents", "close_vcd\\\\n")?;

        Ok(())
    });
    bld.op(
        "xrt",
        &[xilinx_setup, sim_setup, testbench_normal_setup, xrt_setup],
        xclbin,
        dat,
        |e, input, output| {
            e.build_cmd(
                &[output],
                "xclrun",
                &[input, "$sim_data"],
                &["emconfig.json"],
            )?;
            let rsrc_dir = e.config_val("rsrc")?;
            e.arg("xrt_ini", &format!("{}/xrt.ini", rsrc_dir))?;
            Ok(())
        },
    );
    bld.op(
        "xrt-trace",
        &[xilinx_setup, sim_setup, testbench_normal_setup, xrt_setup],
        xclbin,
        vcd,
        |e, input, output| {
            e.build_cmd(
                &[output], // TODO not the VCD, yet...
                "xclrun",
                &[input, "$sim_data"],
                &["emconfig.json", "pre_sim.tcl", "post_sim.tcl"],
            )?;
            let rsrc_dir = e.config_val("rsrc")?;
            e.arg("xrt_ini", &format!("{}/xrt_trace.ini", rsrc_dir))?;
            Ok(())
        },
    );

    bld.build()
}

fn main() -> anyhow::Result<()> {
    let driver = build_driver();

    cli::cli(&driver)
}
