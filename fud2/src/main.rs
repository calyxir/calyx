use fud_core::{
    cli,
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

fn build_driver(bld: &mut DriverBuilder) {
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

        // The Verilog testbench.
        e.rsrc("tb.sv")?;

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

    // Icarus Verilog.
    let verilog_noverify = bld.state("verilog-noverify", &["sv"]);
    let icarus_setup = bld.setup("Icarus Verilog", |e| {
        e.var("iverilog", "iverilog")?;
        e.rule("icarus-compile", "$iverilog -g2012 -o $out tb.sv $in")?;
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
        &[sim_setup, icarus_setup],
        verilog_noverify,
        simulator,
        |e, input, output| {
            e.build_cmd(&[output], "icarus-compile", &[input], &["tb.sv"])?;
            Ok(())
        },
    );

    // Calyx to FIRRTL.
    let firrtl = bld.state("firrtl", &["fir"]);
    bld.op(
        "calyx-to-firrtl",
        &[calyx_setup],
        calyx,
        firrtl,
        |e, input, output| {
            e.build_cmd(&[output], "calyx", &[input], &[])?;
            e.arg("backend", "firrtl")?;
            Ok(())
        },
    );

    // The FIRRTL compiler.
    let firrtl_setup = bld.setup("Firrtl to Verilog compiler", |e| {
        e.config_var("firrtl-exe", "firrtl.exe")?;
        e.rule("firrtl", "$firrtl-exe -i $in -o $out -X sverilog")?;

        e.rsrc("primitives-for-firrtl.sv")?;
        e.rule(
            "add-firrtl-prims",
            "cat primitives-for-firrtl.sv $in > $out",
        )?;

        Ok(())
    });
    fn firrtl_compile(
        e: &mut Emitter,
        input: &str,
        output: &str,
    ) -> EmitResult {
        let tmp_verilog = "partial.sv";
        e.build_cmd(&[tmp_verilog], "firrtl", &[input], &[])?;
        e.build_cmd(
            &[output],
            "add-firrtl-prims",
            &[tmp_verilog],
            &["primitives-for-firrtl.sv"],
        )?;
        Ok(())
    }
    bld.op("firrtl", &[firrtl_setup], firrtl, verilog, firrtl_compile);
    // This is a bit of a hack, but the Icarus-friendly "noverify" state is identical for this path
    // (since FIRRTL compilation doesn't come with verification).
    bld.op(
        "firrtl-noverify",
        &[firrtl_setup],
        firrtl,
        verilog_noverify,
        firrtl_compile,
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
            "$verilator $in tb.sv --trace --binary --top-module TOP -fno-inline -Mdir $out-dir",
        )?;
        e.rule("cp", "cp $in $out")?;
        Ok(())
    });
    bld.op(
        "verilator",
        &[sim_setup, verilator_setup],
        verilog,
        simulator,
        |e, input, output| {
            let out_dir = "verilator-out";
            let sim_bin = format!("{}/VTOP", out_dir);
            e.build_cmd(
                &[&sim_bin],
                "verilator-compile",
                &[input],
                &["tb.sv"],
            )?;
            e.arg("out-dir", out_dir)?;
            e.build("cp", &sim_bin, output)?;
            Ok(())
        },
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
        Ok(())
    });
    bld.op(
        "interp",
        &[sim_setup, calyx_setup, cider_setup],
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
        "debug",
        &[sim_setup, calyx_setup, cider_setup],
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
        &[xilinx_setup, sim_setup, xrt_setup],
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
        &[xilinx_setup, sim_setup, xrt_setup],
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
}

fn main() -> anyhow::Result<()> {
    let mut bld = DriverBuilder::new("fud2");
    build_driver(&mut bld);

    // In debug mode, get resources from the source directory.
    #[cfg(debug_assertions)]
    bld.rsrc_dir(manifest_dir_macros::directory_path!("rsrc"));

    // In release mode, embed resources into the binary.
    #[cfg(not(debug_assertions))]
    bld.rsrc_files({
        const DIR: include_dir::Dir =
            include_dir::include_dir!("$CARGO_MANIFEST_DIR/rsrc");
        DIR.files()
            .map(|file| (file.path().to_str().unwrap(), file.contents()))
            .collect()
    });

    let driver = bld.build();
    cli::cli(&driver)
}

#[cfg(test)]
mod test {
    use crate::build_driver;
    use fud_core::{
        config::default_config, exec::Request, run::Run, Driver, DriverBuilder,
    };

    fn test_driver() -> Driver {
        let mut bld = DriverBuilder::new("fud2");
        build_driver(&mut bld);
        bld.build()
    }

    fn request(
        driver: &Driver,
        start: &str,
        end: &str,
        through: &[&str],
    ) -> Request {
        fud_core::exec::Request {
            start_file: None,
            start_state: driver.get_state(start).unwrap(),
            end_file: None,
            end_state: driver.get_state(end).unwrap(),
            through: through
                .iter()
                .map(|s| driver.get_op(s).unwrap())
                .collect(),
            workdir: ".".into(),
        }
    }

    fn emit_ninja(driver: &Driver, req: Request) -> String {
        let plan = driver.plan(req).unwrap();
        let config = default_config().merge(("calyx.base", "/test/calyx"));
        let run = Run::with_config(driver, plan, config);
        let mut buf = vec![];
        run.emit(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn test_emit(start: &str, end: &str, through: &[&str]) -> String {
        let driver = test_driver();
        let req = request(&driver, start, end, through);
        emit_ninja(&driver, req)
    }

    #[test]
    fn test_something() {
        let ninja = test_emit("calyx", "verilog", &[]);
        insta::assert_snapshot!(ninja);
    }
}
