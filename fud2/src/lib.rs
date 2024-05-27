use fud_core::{
    exec::{SetupRef, StateRef},
    run::{EmitResult, Emitter},
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

pub fn build_driver(bld: &mut DriverBuilder) {
    // The verilog state
    let verilog = bld.state("verilog", &["sv", "v"]);
    // Calyx.
    let (calyx, calyx_setup) = setup_calyx(bld, verilog);
    // Dahlia.
    setup_dahlia(bld, calyx);
    // MrXL.
    setup_mrxl(bld, calyx);

    // Shared machinery for RTL simulators.
    let dat = bld.state("dat", &["json"]);
    let vcd = bld.state("vcd", &["vcd"]);
    let simulator = bld.state("sim", &["exe"]);
    let sim_setup = bld.setup("RTL simulation", |e| {
        // Data conversion to and from JSON.
        e.config_var_or("python", "python", "python3")?;
        e.rsrc("json-dat.py")?;
        e.rule("hex-data", "$python json-dat.py --from-json $in $out")?;
        e.rule("json-data", "$python json-dat.py --to-json $out $in")?;

        // The input data file. `sim.data` is required.
        let data_name = e.config_val("sim.data")?;
        let data_path = e.external_path(data_name.as_ref());
        e.var("sim_data", data_path.as_str())?;

        // Produce the data directory.
        e.var("datadir", "sim_data")?;
        e.build_cmd(
            &["$datadir"],
            "hex-data",
            &["$sim_data"],
            &["json-dat.py"],
        )?;

        // Rule for simulation execution.
        e.rule(
            "sim-run",
            "./$bin +DATA=$datadir +CYCLE_LIMIT=$cycle-limit $args > $out",
        )?;

        // More shared configuration.
        e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000")?;

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
            e.build_cmd(
                &[output],
                "json-data",
                &["$datadir", "sim.log"],
                &["json-dat.py"],
            )?;
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

    // Creating custom testbench
    let verilog_refmem = bld.state("verilog-refmem", &["sv"]);
    let verilog_refmem_noverify = bld.state("verilog-refmem-noverify", &["sv"]);

    // Icarus Verilog.
    let verilog_noverify = bld.state("verilog-noverify", &["sv"]);
    let icarus_setup = bld.setup("Icarus Verilog", |e| {
        e.var("iverilog", "iverilog")?;
        e.rule(
            "icarus-compile",
            "$iverilog -g2012 -o $out $testbench $additional-input $in",
        )?;
        Ok(())
    });
    // [Should be default] Setup for using tb.sv as testbench (and managing memories within the design)
    let testbench_normal_setup = bld.setup("Normal Testbench Setup", |e| {
        // The Verilog testbench.
        e.var("testbench", &format!("{}/tb.sv", e.config_val("rsrc")?))?;
        Ok(())
    });
    // [Needs YXI backend compiled] Setup for creating a custom testbench (needed for FIRRTL)
    let testbench_refmem_setup = bld.setup("Custom Testbench Setup", |e| {
        // Convert all ref cells to @external (FIXME: YXI should work for both?)
        e.rule("ref-to-external", "sed 's/ref /@external /g' $in > $out")?;

        // Convert all @external cells to ref (FIXME: we want to deprecate @external)
        e.rule("external-to-ref", "sed 's/@external([0-9]*)/ref/g' $in | sed 's/@external/ref/g' > $out")?;

        // Produce a custom testbench that handles memory reading and writing.
        e.var("testbench", "refmem_tb.sv")?;
        e.var(
            "gen-testbench-script",
            "$calyx-base/tools/firrtl/generate-testbench.py",
        )?;
        e.var(
            "additional-input",
            &format!("{}/memories.sv", e.config_val("rsrc")?),
        )?;

        e.rule(
            "generate-refmem-testbench",
            "python3 $gen-testbench-script $in | tee $testbench $out",
        )?;

        // dummy rule to force ninja to build the testbench
        e.var(
            "dummy-script",
            &format!("{}/dummy.sh", e.config_val("rsrc")?),
        )?;
        e.rule("dummy", "bash $dummy-script $in > $out")?;

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
            e.build_cmd(&[output], "icarus-compile", &[input], &[])?;
            Ok(())
        },
    );
    bld.op(
        "icarus-refmem",
        &[sim_setup, icarus_setup],
        verilog_refmem_noverify,
        simulator,
        |e, input, output| {
            // FIXME: remove code clone
            e.build_cmd(&[output], "icarus-compile", &[input], &[])?;
            Ok(())
        },
    );

    // setup for FIRRTL-implemented primitives
    let firrtl_primitives_setup = bld.setup("FIRRTL with primitives", |e| {
        // Produce FIRRTL with FIRRTL-defined primitives.
        e.var(
            "gen-firrtl-primitives-script",
            "$calyx-base/tools/firrtl/generate-firrtl-with-primitives.py",
        )?;
        e.rule(
            "generate-firrtl-with-primitives",
            "python3 $gen-firrtl-primitives-script $in > $out",
        )?;

        Ok(())
    });

    fn calyx_to_firrtl_helper(
        e: &mut Emitter,
        input: &str,
        output: &str,
        firrtl_primitives: bool, // Use FIRRTL primitive implementations?
    ) -> EmitResult {
        // Temporary Calyx where all refs are converted into external (FIXME: fix YXI to emit for ref as well?)
        let only_externals_calyx = "external.futil";
        // Temporary Calyx where all externals are converted into refs (for FIRRTL backend)
        let only_refs_calyx = "ref.futil";
        // JSON with memory information created by YXI
        let memories_json = "memory-info.json";
        // testbench creation needs a file to output to
        let dummy_testbench = "refmem-tb-copy.sv";
        // Holds contents of file we want to output. Gets cat-ed via final dummy command
        let tmp_out = "tmp-out.fir";
        // Convert ref into external to get YXI working (FIXME: fix YXI to emit for ref as well?)
        e.build_cmd(&[only_externals_calyx], "ref-to-external", &[input], &[])?;
        // Convert external to ref to get FIRRTL backend working
        e.build_cmd(&[only_refs_calyx], "external-to-ref", &[input], &[])?;

        // Get YXI to generate JSON for testbench generation
        e.build_cmd(&[memories_json], "calyx", &[only_externals_calyx], &[])?;
        e.arg("backend", "yxi")?;
        // generate custom testbench
        e.build_cmd(
            &[dummy_testbench],
            "generate-refmem-testbench",
            &[memories_json],
            &[],
        )?;

        if firrtl_primitives {
            let core_program_firrtl = "core.fir";

            // Obtain FIRRTL of core program
            e.build_cmd(
                &[core_program_firrtl],
                "calyx",
                &[only_refs_calyx],
                &[],
            )?;
            e.arg("backend", "firrtl")?;
            e.arg("args", "--synthesis")?;

            // Obtain primitive uses JSON for metaprogramming
            let primitive_uses_json = "primitive-uses.json";
            e.build_cmd(
                &[primitive_uses_json],
                "calyx",
                &[only_refs_calyx],
                &[],
            )?;
            e.arg("backend", "primitive-uses")?;
            e.arg("args", "--synthesis")?;

            // run metaprogramming script to get FIRRTL with primitives
            e.build_cmd(
                &[tmp_out],
                "generate-firrtl-with-primitives",
                &[core_program_firrtl, primitive_uses_json],
                &[],
            )?;
        } else {
            // emit extmodule declarations to use Verilog primitive implementations
            e.build_cmd(&[tmp_out], "calyx", &[only_refs_calyx], &[])?;
            e.arg("backend", "firrtl")?;
            e.arg("args", "--emit-primitive-extmodules")?;
        }

        // dummy command to make sure custom testbench is created but not emitted as final answer
        e.build_cmd(&[output], "dummy", &[tmp_out, dummy_testbench], &[])?;

        Ok(())
    }

    // Calyx to FIRRTL.
    let firrtl = bld.state("firrtl", &["fir"]); // using Verilog primitives
    let firrtl_with_primitives = bld.state("firrtl-with-primitives", &["fir"]); // using FIRRTL primitives
    bld.op(
        // use Verilog
        "calyx-to-firrtl",
        &[calyx_setup, testbench_refmem_setup],
        calyx,
        firrtl,
        |e, input, output| calyx_to_firrtl_helper(e, input, output, false),
    );

    bld.op(
        "firrtl-with-primitives",
        &[calyx_setup, firrtl_primitives_setup, testbench_refmem_setup],
        calyx,
        firrtl_with_primitives,
        |e, input, output| calyx_to_firrtl_helper(e, input, output, true),
    );

    // The FIRRTL compiler.
    let firrtl_setup = bld.setup("Firrtl to Verilog compiler", |e| {
        e.config_var("firrtl-exe", "firrtl.exe")?;
        e.rule("firrtl", "$firrtl-exe -i $in -o $out -X sverilog")?;

        e.rsrc("primitives-for-firrtl.sv")?;
        // adding Verilog implementations of primitives to FIRRTL --> Verilog compiled code
        e.rule(
            "add-verilog-primitives",
            "cat primitives-for-firrtl.sv $in > $out",
        )?;

        Ok(())
    });

    fn firrtl_compile_helper(
        e: &mut Emitter,
        input: &str,
        output: &str,
        firrtl_primitives: bool,
    ) -> EmitResult {
        if firrtl_primitives {
            e.build_cmd(&[output], "firrtl", &[input], &[])?;
        } else {
            let tmp_verilog = "partial.sv";
            e.build_cmd(&[tmp_verilog], "firrtl", &[input], &[])?;
            e.build_cmd(
                &[output],
                "add-verilog-primitives",
                &[tmp_verilog],
                &["primitives-for-firrtl.sv"],
            )?;
        }
        Ok(())
    }
    // FIRRTL --> Verilog compilation using Verilog primitive implementations for Verilator
    bld.op(
        "firrtl",
        &[firrtl_setup],
        firrtl,
        verilog_refmem,
        |e, input, output| firrtl_compile_helper(e, input, output, false),
    );
    // FIRRTL --> Verilog compilation using Verilog primitive implementations for Icarus
    // This is a bit of a hack, but the Icarus-friendly "noverify" state is identical for this path
    // (since FIRRTL compilation doesn't come with verification).
    bld.op(
        "firrtl-noverify",
        &[firrtl_setup],
        firrtl,
        verilog_refmem_noverify,
        |e, input, output| firrtl_compile_helper(e, input, output, false),
    );
    // FIRRTL --> Verilog compilation using FIRRTL primitive implementations for Verilator
    bld.op(
        "firrtl-with-primitives",
        &[firrtl_setup],
        firrtl_with_primitives,
        verilog_refmem,
        |e, input, output| firrtl_compile_helper(e, input, output, true),
    );
    // FIRRTL --> Verilog compilation using FIRRTL primitive implementations for Icarus
    bld.op(
        "firrtl-with-primitives-noverify",
        &[firrtl_setup],
        firrtl_with_primitives,
        verilog_refmem_noverify,
        |e, input, output| firrtl_compile_helper(e, input, output, true),
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
            "$verilator $in $testbench $additional-input --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
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
        e.build_cmd(&[&sim_bin], "verilator-compile", &[input], &[])?;
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
        &[sim_setup, testbench_refmem_setup, verilator_setup],
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
        e.config_var_or(
            "cider-converter",
            "cider-converter.exe",
            "$calyx-base/target/debug/cider-data-converter",
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

        // TODO Can we reduce the duplication around and `$python`?
        e.rsrc("interp-dat.py")?;
        e.config_var_or("python", "python", "python3")?;
        e.rule("dat-to-interp", "$python interp-dat.py --to-interp $in")?;
        e.rule(
            "interp-to-dat",
            "$python interp-dat.py --from-interp $in $sim_data > $out",
        )?;
        e.build_cmd(
            &["data.json"],
            "dat-to-interp",
            &["$sim_data"],
            &["interp-dat.py"],
        )?;

        e.rule(
            "cider2",
            "$cider-exe -l $calyx-base --data data.dump $in flat > $out",
        )?;

        e.rule("dump-to-interp", "$cider-converter --to cider $in > $out")?;
        e.rule("interp-to-dump", "$cider-converter --to json $in > $out")?;
        e.build_cmd(
            &["data.dump"],
            "dump-to-interp",
            &["$sim_data"],
            &["$cider-converter"],
        )?;

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
                &["$sim_data", "interp-dat.py"],
            )?;
            Ok(())
        },
    );
    bld.op(
        "interp-flat",
        &[sim_setup, calyx_setup, cider_setup],
        calyx,
        dat,
        |e, input, output| {
            let out_file = "interp_out.dump";
            e.build_cmd(&[out_file], "cider2", &[input], &["data.dump"])?;
            e.build_cmd(
                &[output],
                "interp-to-dump",
                &[out_file],
                &["$sim_data", "$cider-converter"],
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
        e.rsrc("gen_xo.tcl")?;
        e.rsrc("get-ports.py")?;
        e.config_var_or("python", "python", "python3")?;
        e.rule("gen-xo", "$vivado-dir/bin/vivado -mode batch -source gen_xo.tcl -tclargs $out `$python get-ports.py kernel.xml`")?;
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
                &[
                    "main.sv",
                    "toplevel.v",
                    "kernel.xml",
                    "gen_xo.tcl",
                    "get-ports.py",
                ],
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
            e.rsrc("xrt.ini")?;
            e.build_cmd(
                &[output],
                "xclrun",
                &[input, "$sim_data"],
                &["emconfig.json", "xrt.ini"],
            )?;
            e.arg("xrt_ini", "xrt.ini")?;
            Ok(())
        },
    );
    bld.op(
        "xrt-trace",
        &[xilinx_setup, sim_setup, testbench_normal_setup, xrt_setup],
        xclbin,
        vcd,
        |e, input, output| {
            e.rsrc("xrt_trace.ini")?;
            e.build_cmd(
                &[output], // TODO not the VCD, yet...
                "xclrun",
                &[input, "$sim_data"],
                &[
                    "emconfig.json",
                    "pre_sim.tcl",
                    "post_sim.tcl",
                    "xrt_trace.ini",
                ],
            )?;
            e.arg("xrt_ini", "xrt_trace.ini")?;
            Ok(())
        },
    );

    let yxi = bld.state("yxi", &["yxi"]);
    bld.op(
        "calyx-to-yxi",
        &[calyx_setup],
        calyx,
        yxi,
        |e, input, output| {
            e.build_cmd(&[output], "calyx", &[input], &[])?;
            e.arg("backend", "yxi")?;
            Ok(())
        },
    );

    let wrapper_setup = bld.setup("YXI and AXI generation", |e| {
        // Define a `gen-axi` rule that invokes our Python code generator program.
        // For now point to standalone axi-generator.py. Can maybe turn this into a rsrc file?
        e.config_var_or(
            "axi-generator",
            "axi.generator",
            "$calyx-base/yxi/axi-calyx/axi-generator.py",
        )?;
        e.config_var_or("python", "python", "python3")?;
        e.rule("gen-axi", "$python $axi-generator $in > $out")?;

        // Define a simple `combine` rule that just concatenates any numer of files.
        e.rule("combine", "cat $in > $out")?;

        e.rule(
            "remove-imports",
            "sed '1,/component main/{/component main/!d}' $in > $out",
        )?;
        Ok(())
    });
    bld.op(
        "axi-wrapped",
        &[calyx_setup, wrapper_setup],
        calyx,
        calyx,
        |e, input, output| {
            // Generate the YXI file.
            //no extension
            let file_name = input
                .rsplit_once('/')
                .unwrap()
                .1
                .rsplit_once('.')
                .unwrap()
                .0;
            let tmp_yxi = format!("{}.yxi", file_name);

            //Get yxi file from main compute program.
            //TODO(nate): Can this use the `yxi` operation instead of hardcoding the build cmd calyx rule with arguments?
            e.build_cmd(&[&tmp_yxi], "calyx", &[input], &[])?;
            e.arg("backend", "yxi")?;

            // Generate the AXI wrapper.
            let refified_calyx = format!("refified_{}.futil", file_name);
            e.build_cmd(&[&refified_calyx], "calyx-pass", &[input], &[])?;
            e.arg("pass", "external-to-ref")?;

            let axi_wrapper = "axi_wrapper.futil";
            e.build_cmd(&[axi_wrapper], "gen-axi", &[&tmp_yxi], &[])?;

            // Generate no-imports version of the refified calyx.
            let no_imports_calyx = format!("no_imports_{}", refified_calyx);
            e.build_cmd(
                &[&no_imports_calyx],
                "remove-imports",
                &[&refified_calyx],
                &[],
            )?;

            // Combine the original Calyx and the wrapper.
            e.build_cmd(
                &[output],
                "combine",
                &[axi_wrapper, &no_imports_calyx],
                &[],
            )?;
            Ok(())
        },
    );
}
