let xo = state("xo", ["xo"]);
let xclbin = state("xclbin", ["xclbin"]);

fn xilinx_setup(e) {
    // Locations for Vivado and Vitis installations.
    e.config_var("vivado-dir", "xilinx.vivado");
    e.config_var("vitis-dir", "xilinx.vitis");

    // Package a Verilog program as an `.xo` file.
    e.rsrc("gen_xo.tcl");
    e.rsrc("get-ports.py");
    e.config_var_or("python", "python", "python3");
    e.rule(
        "gen-xo",
        "$vivado-dir/bin/vivado -mode batch -source gen_xo.tcl -tclargs $out `$python get-ports.py kernel.xml`"
    );
    e.arg("pool", "console");  // Lets Ninja stream the tool output "live."

    // Compile an `.xo` file to an `.xclbin` file, which is where the actual EDA work occurs.
    e.config_var_or("xilinx-mode", "xilinx.mode", "hw_emu");
    e.config_var_or("platform", "xilinx.device", "xilinx_u50_gen3x16_xdma_201920_3");
    e.rule(
        "compile-xclbin",
        "$vitis-dir/bin/v++ -g -t $xilinx-mode --platform $platform --save-temps --profile.data all:all:all --profile.exec all:all:all -lo $out $in"
    );
    e.arg("pool", "console");
};

let calyx = get_state("calyx");
let calyx_setup = get_setup("Calyx compiler");

op(
    "xo2",
    [calyx_setup, xilinx_setup],
    calyx,
    xo,
    |e, input, output| {
        // Emit the Verilog itself in "synthesis mode."
        e.build_cmd(["main.sv"], "calyx", [input], []);
        e.arg("backend", "verilog");
        e.arg("args", "--synthesis -p external");

        // Extra ingredients for the `.xo` package.
        e.build_cmd(["toplevel.v"], "calyx", [input], []);
        e.arg("backend", "xilinx");
        e.build_cmd(["kernel.xml"], "calyx", [input], []);
        e.arg("backend", "xilinx-xml");

        // Package the `.xo`.
        e.build_cmd(
            [output],
            "gen-xo",
            [],
            [
               "main.sv",
               "toplevel.v",
               "kernel.xml",
               "gen_xo.tcl",
               "get-ports.py",
            ],
        );
    },
);

op("xclbin2", [xilinx_setup], xo, xclbin, |e, input, output| {
    e.build_cmd([output], "compile-xclbin", [input], []);
});

let tb_setup = get_setup("Standalone Testbench Setup");
let sim_setup = get_setup("RTL simulation");

// Xilinx execution.
// TODO Only does `hw_emu` for now...
fn xrt_setup(e) {
    // Generate `emconfig.json`.
    e.rule("emconfig", "$vitis-dir/bin/emconfigutil --platform $platform");
    e.build_cmd(["emconfig.json"], "emconfig", [], []);

    // Execute via the `xclrun` tool.
    e.config_var("xrt-dir", "xilinx.xrt");
    e.rule(
        "xclrun",
        "bash -c 'source $vitis-dir/settings64.sh ; source $xrt-dir/setup.sh ; XRT_INI_PATH=$xrt_ini EMCONFIG_PATH=. XCL_EMULATION_MODE=$xilinx-mode $python -m fud.xclrun --out $out $in'"
    );
    e.arg("pool", "console");

    // "Pre-sim" and "post-sim" scripts for simulation.
    e.rule("echo", "echo $contents > $out");
    e.build_cmd(["pre_sim.tcl"], "echo", [""], [""]);
    e.arg("contents", "open_vcd\\\\nlog_vcd *\\\\n");
    e.build_cmd(["post_sim.tcl"], "echo", [""], [""]);
    e.arg("contents", "close_vcd\\\\n");
};

op(
    "xrt2",
    [
        xilinx_setup,
        sim_setup,
        tb_setup,
        xrt_setup,
    ],
    xclbin,
    get_state("dat"),
    |e, input, output| {
        e.rsrc("xrt.ini");
        e.build_cmd(
            [output],
            "xclrun",
            [input, "$sim_data"],
            ["emconfig.json", "xrt.ini"],
        );
        e.arg("xrt_ini", "xrt.ini");
    },
);

op(
    "xrt-trace2",
    [
        xilinx_setup,
        sim_setup,
        tb_setup,
        xrt_setup,
    ],
    xclbin,
    get_state("vcd"),
    |e, input, output| {
        e.rsrc("xrt_trace.ini");
        e.build_cmd(
            [output], // TODO not the VCD, yet...
            "xclrun",
            [input, "$sim_data"],
            [
                "emconfig.json",
                "pre_sim.tcl",
                "post_sim.tcl",
                "xrt_trace.ini",
            ],
        );
        e.arg("xrt_ini", "xrt_trace.ini");
    },
);
