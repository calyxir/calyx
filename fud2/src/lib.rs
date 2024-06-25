use std::str::FromStr;

use fud_core::{
    exec::{SetupRef, StateRef},
    run::{EmitResult, StreamEmitter},
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
            e.build_cmd(&["sim.log"], "sim-run", &[input[0], "$datadir"], &[])?;
            e.arg("bin", input[0])?;
            e.arg("args", "+NOTRACE=1")?;
            e.build_cmd(
                &[output[0]],
                "json-data",
                &["$datadir", "sim.log"],
                &["json-dat.py"],
            )?;
            Ok(())
        },
    );
    bld.op("trace", &[sim_setup], simulator, vcd, |e, input, output| {
        e.build_cmd(
            &["sim.log", output[0]],
            "sim-run",
            &[input[0], "$datadir"],
            &[],
        )?;
        e.arg("bin", input[0])?;
        e.arg("args", &format!("+NOTRACE=0 +OUT={}", output[0]))?;
        Ok(())
    });

    // The "verilog_refmem" states are variants of the other Verilog states that use the external testbench style.
    // "refmem" refers to the fact that their memories are external, meaning that they need to be linked with
    // a testbench that will provide those memories.
    let verilog_refmem = bld.state("verilog-refmem", &["sv"]);
    let verilog_refmem_noverify = bld.state("verilog-refmem-noverify", &["sv"]);

    // Icarus Verilog.
    let verilog_noverify = bld.state("verilog-noverify", &["sv", "v"]);
    let icarus_setup = bld.setup("Icarus Verilog", |e| {
        e.var("iverilog", "iverilog")?;
        e.rule(
            "icarus-compile-standalone-tb",
            "$iverilog -g2012 -o $out tb.sv $in",
        )?;
        e.rule(
            "icarus-compile-custom-tb",
            "$iverilog -g2012 -o $out tb.sv memories.sv $in",
        )?;
        Ok(())
    });
    // [Default] Setup for using rsrc/tb.sv as testbench (and managing memories within the design)
    let standalone_testbench_setup =
        bld.setup("Standalone Testbench Setup", |e| {
            // Standalone Verilog testbench.
            e.rsrc("tb.sv")?;

            Ok(())
        });
    // [Needs YXI backend compiled] Setup for creating a custom testbench (needed for FIRRTL)
    let custom_testbench_setup = bld.setup("Custom Testbench Setup", |e| {
        // Convert all ref cells to @external (FIXME: YXI should work for both?)
        e.rule("ref-to-external", "sed 's/ref /@external /g' $in > $out")?;

        // Convert all @external cells to ref (FIXME: we want to deprecate @external)
        e.rule("external-to-ref", "sed 's/@external([0-9]*)/ref/g' $in | sed 's/@external/ref/g' > $out")?;

        e.var(
            "gen-testbench-script",
            "$calyx-base/tools/firrtl/generate-testbench.py",
        )?;
        e.rsrc("memories.sv")?; // Memory primitives.

        e.rule(
            "generate-refmem-testbench",
            "python3 $gen-testbench-script $in > $out",
        )?;

        // dummy rule to force ninja to build the testbench
        e.rule("dummy", "sh -c 'cat $$0' $in > $out")?;

        Ok(())
    });
    bld.op(
        "calyx-noverify",
        &[calyx_setup],
        calyx,
        verilog_noverify,
        |e, input, output| {
            // Icarus requires a special --disable-verify version of Calyx code.
            e.build_cmd(&[output[0]], "calyx", &[input[0]], &[])?;
            e.arg("backend", "verilog")?;
            e.arg("args", "--disable-verify")?;
            Ok(())
        },
    );

    bld.op(
        "icarus",
        &[sim_setup, standalone_testbench_setup, icarus_setup],
        verilog_noverify,
        simulator,
        |e, input, output| {
            e.build_cmd(
                &[output[0]],
                "icarus-compile-standalone-tb",
                &[input[0]],
                &["tb.sv"],
            )?;
            Ok(())
        },
    );
    bld.op(
        "icarus-refmem",
        &[sim_setup, icarus_setup],
        verilog_refmem_noverify,
        simulator,
        |e, input, output| {
            e.build_cmd(
                &[output[0]],
                "icarus-compile-custom-tb",
                &[input[0]],
                &["tb.sv", "memories.sv"],
            )?;
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
        e: &mut StreamEmitter,
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
        // Custom testbench (same name as standalone testbench)
        let testbench = "tb.sv";
        // Holds contents of file we want to output. Gets cat-ed via final dummy command
        let tmp_out = "tmp-out.fir";
        // Convert ref into external to get YXI working (FIXME: fix YXI to emit for ref as well?)
        e.build_cmd(&[only_externals_calyx], "ref-to-external", &[input], &[])?;
        // Convert external to ref to get FIRRTL backend working
        e.build_cmd(&[only_refs_calyx], "external-to-ref", &[input], &[])?;

        // Get YXI to generate JSON for testbench generation
        e.build_cmd(&[memories_json], "yxi", &[only_externals_calyx], &[])?;
        // generate custom testbench
        e.build_cmd(
            &[testbench],
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
        e.build_cmd(&[output], "dummy", &[tmp_out, testbench], &[])?;

        Ok(())
    }

    // Calyx to FIRRTL.
    let firrtl = bld.state("firrtl", &["fir"]); // using Verilog primitives
    let firrtl_with_primitives = bld.state("firrtl-with-primitives", &["fir"]); // using FIRRTL primitives
    bld.op(
        // use Verilog
        "calyx-to-firrtl",
        &[calyx_setup, custom_testbench_setup],
        calyx,
        firrtl,
        |e, input, output| {
            calyx_to_firrtl_helper(e, input[0], output[0], false)
        },
    );

    bld.op(
        "firrtl-with-primitives",
        &[calyx_setup, firrtl_primitives_setup, custom_testbench_setup],
        calyx,
        firrtl_with_primitives,
        |e, input, output| calyx_to_firrtl_helper(e, input[0], output[0], true),
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
        e: &mut StreamEmitter,
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
        |e, input, output| firrtl_compile_helper(e, input[0], output[0], false),
    );
    // FIRRTL --> Verilog compilation using Verilog primitive implementations for Icarus
    // This is a bit of a hack, but the Icarus-friendly "noverify" state is identical for this path
    // (since FIRRTL compilation doesn't come with verification).
    bld.op(
        "firrtl-noverify",
        &[firrtl_setup],
        firrtl,
        verilog_refmem_noverify,
        |e, input, output| firrtl_compile_helper(e, input[0], output[0], false),
    );
    // FIRRTL --> Verilog compilation using FIRRTL primitive implementations for Verilator
    bld.op(
        "firrtl-with-primitives",
        &[firrtl_setup],
        firrtl_with_primitives,
        verilog_refmem,
        |e, input, output| firrtl_compile_helper(e, input[0], output[0], true),
    );
    // FIRRTL --> Verilog compilation using FIRRTL primitive implementations for Icarus
    bld.op(
        "firrtl-with-primitives-noverify",
        &[firrtl_setup],
        firrtl_with_primitives,
        verilog_refmem_noverify,
        |e, input, output| firrtl_compile_helper(e, input[0], output[0], true),
    );

    // primitive-uses backend
    let primitive_uses_json = bld.state("primitive-uses-json", &["json"]);
    bld.op(
        "primitive-uses",
        &[calyx_setup],
        calyx,
        primitive_uses_json,
        |e, input, output| {
            e.build_cmd(&[output[0]], "calyx", &[input[0]], &[])?;
            e.arg("backend", "primitive-uses")?;
            Ok(())
        },
    );

    // Verilator.
    let verilator_setup = bld.setup("Verilator", |e| {
        e.config_var_or("verilator", "verilator.exe", "verilator")?;
        e.config_var_or("cycle-limit", "sim.cycle_limit", "500000000")?;
        e.rule(
            "verilator-compile-standalone-tb",
            "$verilator $in tb.sv --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
        )?;
        e.rule(
            "verilator-compile-custom-tb",
            "$verilator $in tb.sv memories.sv --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
        )?;
        e.rule("cp", "cp $in $out")?;
        Ok(())
    });
    fn verilator_build(
        e: &mut StreamEmitter,
        input: &str,
        output: &str,
        standalone_testbench: bool,
    ) -> EmitResult {
        let out_dir = "verilator-out";
        let sim_bin = format!("{}/VTOP", out_dir);
        if standalone_testbench {
            e.build_cmd(
                &[&sim_bin],
                "verilator-compile-standalone-tb",
                &[input],
                &["tb.sv"],
            )?;
        } else {
            e.build_cmd(
                &[&sim_bin],
                "verilator-compile-custom-tb",
                &[input],
                &["tb.sv", "memories.sv"],
            )?;
        }
        e.arg("out-dir", out_dir)?;
        e.build("cp", &sim_bin, output)?;
        Ok(())
    }

    bld.op(
        "verilator",
        &[sim_setup, standalone_testbench_setup, verilator_setup],
        verilog,
        simulator,
        |e, input, output| verilator_build(e, input[0], output[0], true),
    );

    bld.op(
        "verilator-refmem",
        &[sim_setup, custom_testbench_setup, verilator_setup],
        verilog_refmem,
        simulator,
        |e, input, output| verilator_build(e, input[0], output[0], false),
    );

    // Interpreter.
    let debug = bld.state("debug", &[]); // A pseudo-state.
                                         // A pseudo-state for cider input
    let cider_state = bld.state("cider", &[]);

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
            "run-cider",
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
        "calyx-to-cider",
        &[sim_setup, calyx_setup],
        calyx,
        cider_state,
        |e, input, _output| {
            e.build_cmd(
                &["cider-input.futil"],
                "calyx-with-flags",
                input,
                &[],
            )?;
            Ok(())
        },
    );

    bld.op(
        "interp",
        &[
            sim_setup,
            standalone_testbench_setup,
            calyx_setup,
            cider_setup,
        ],
        calyx,
        dat,
        |e, input, output| {
            let out_file = "interp_out.json";
            e.build_cmd(&[out_file], "cider", &[input[0]], &["data.json"])?;
            e.build_cmd(
                &[output[0]],
                "interp-to-dat",
                &[out_file],
                &["$sim_data", "interp-dat.py"],
            )?;
            Ok(())
        },
    );
    bld.op(
        "cider",
        &[sim_setup, calyx_setup, cider_setup],
        cider_state,
        dat,
        |e, _input, output| {
            let out_file = "interp_out.dump";
            e.build_cmd(
                &[out_file],
                "run-cider",
                &["cider-input.futil"],
                &["data.dump"],
            )?;
            e.build_cmd(
                &[output[0]],
                "interp-to-dump",
                &[out_file],
                &["$sim_data", "$cider-converter"],
            )?;
            Ok(())
        },
    );
    bld.op(
        "debug",
        &[
            sim_setup,
            standalone_testbench_setup,
            calyx_setup,
            cider_setup,
        ],
        calyx,
        debug,
        |e, input, output| {
            e.build_cmd(
                &[output[0]],
                "cider-debug",
                &[input[0]],
                &["data.json"],
            )?;
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
            e.build_cmd(&["main.sv"], "calyx", &[input[0]], &[])?;
            e.arg("backend", "verilog")?;
            e.arg("args", "--synthesis -p external")?;

            // Extra ingredients for the `.xo` package.
            e.build_cmd(&["toplevel.v"], "calyx", &[input[0]], &[])?;
            e.arg("backend", "xilinx")?;
            e.build_cmd(&["kernel.xml"], "calyx", &[input[0]], &[])?;
            e.arg("backend", "xilinx-xml")?;

            // Package the `.xo`.
            e.build_cmd(
                &[output[0]],
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
        e.build_cmd(&[output[0]], "compile-xclbin", &[input[0]], &[])?;
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
        &[
            xilinx_setup,
            sim_setup,
            standalone_testbench_setup,
            xrt_setup,
        ],
        xclbin,
        dat,
        |e, input, output| {
            e.rsrc("xrt.ini")?;
            e.build_cmd(
                &[output[0]],
                "xclrun",
                &[input[0], "$sim_data"],
                &["emconfig.json", "xrt.ini"],
            )?;
            e.arg("xrt_ini", "xrt.ini")?;
            Ok(())
        },
    );
    bld.op(
        "xrt-trace",
        &[
            xilinx_setup,
            sim_setup,
            standalone_testbench_setup,
            xrt_setup,
        ],
        xclbin,
        vcd,
        |e, input, output| {
            e.rsrc("xrt_trace.ini")?;
            e.build_cmd(
                &[output[0]], // TODO not the VCD, yet...
                "xclrun",
                &[input[0], "$sim_data"],
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

    let yxi_setup = bld.setup("YXI setup", |e| {
        e.config_var_or("yxi", "yxi", "$calyx-base/target/debug/yxi")?;
        e.rule("yxi", "$yxi -l $calyx-base $in > $out")?;
        Ok(())
    });

    let yxi = bld.state("yxi", &["yxi"]);
    bld.op(
        "calyx-to-yxi",
        &[calyx_setup, yxi_setup],
        calyx,
        yxi,
        |e, input, output| {
            e.build_cmd(output, "yxi", input, &[])?;
            Ok(())
        },
    );

    let wrapper_setup = bld.setup("YXI and AXI generation", |e| {
        // Define a `gen-axi` rule that invokes our Python code generator program.
        // For now point to standalone axi-generator.py. Can maybe turn this into a rsrc file?
        let dynamic =
            e.config_constrained_or("dynamic", vec!["true", "false"], "false")?;
        let generator_path = if FromStr::from_str(&dynamic)
            .expect("The dynamic flag should be either 'true' or 'false'.")
        {
            "$calyx-base/yxi/axi-calyx/dynamic-axi-generator.py"
        } else {
            "$calyx-base/yxi/axi-calyx/axi-generator.py"
        };
        e.config_var_or("axi-generator", "axi.generator", generator_path)?;
        e.config_var_or("python", "python", "python3")?;

        e.rule("gen-axi", "$python $axi-generator $in > $out")?;

        // Define a simple `combine` rule that just concatenates any numer of files.
        e.rule("combine", "cat $in > $out")?;

        e.rule(
            "remove-imports",
            "sed '1,/component main/{/component main/!d; }' $in > $out",
        )?;
        Ok(())
    });
    bld.op(
        "axi-wrapped",
        &[calyx_setup, yxi_setup, wrapper_setup],
        calyx,
        calyx,
        |e, input, output| {
            // Generate the YXI file.
            // no extension
            let file_name = basename(input[0]);

            // Get yxi file from main compute program.
            let tmp_yxi = format!("{}.yxi", file_name);
            e.build_cmd(&[&tmp_yxi], "yxi", input, &[])?;

            // Generate the AXI wrapper.
            let refified_calyx = format!("refified_{}.futil", file_name);
            e.build_cmd(&[&refified_calyx], "calyx-pass", &[input[0]], &[])?;
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
                &[output[0]],
                "combine",
                &[axi_wrapper, &no_imports_calyx],
                &[],
            )?;
            Ok(())
        },
    );

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
