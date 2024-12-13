import "calyx" as c;
import "verilator" as v;
import "rtl_sim" as sim;

export const instrumented_verilog = state("verilog-instrumented", ["sv"]);
export const instrumented_sim = state("sim-instrumented", ["exe"]);
export const instrumented_vcd = state("vcd-instrumented", ["vcd"]);
export const flamegraph = state("flamegraph", ["svg"]);

fn profiling_setup(e) {
    e.var_("cells", "cells.json");
    e.var_("tdcc-json", "fsm.json"); // might not be necessary if we get rid of fsms?

    // series of passes after instrumentation?
    e.config_var_or("passes", "profiler.compilation-passes", "all"); // set passes="no-opt" to run without optimizations

    // rules for preprocessing

    e.config_var_or("component_cells", "component_cells", "$calyx-base/target/debug/component_cells");
    e.rule("component-cells", "$component_cells -l $calyx-base $in > $out");

    // rules for postprocessing

    // script to process vcd and produce folded representation of flame graph
    e.var_("parse-vcd-script", "$calyx-base/tools/profiler/profiler-process.py");
    e.rule("parse-vcd", "python3 $parse-vcd-script $in $cells profiler-out $out");

    e.config_var("flamegraph-script", "flamegraph.script");
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.sh");
    e.rule("create-visuals", "bash $create-visuals-script $flamegraph-script profiler-out $in $out");

    // Standalone Verilog testbench. copied from testbench
    e.rsrc("tb.sv");

}

fn calyx_to_flamegraph(e, input, output) {
    // instrument calyx and produce verilog
    let instrumented_verilog = "instrumented.sv";
    e.build_cmd(["$cells"], "component-cells", [input], []);
    e.build_cmd([instrumented_verilog], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", " -p static-inline -p compile-static -p compile-repeat -p compile-invoke -p profiler-instrumentation -p $passes -x tdcc:dump-fsm-json=fsm.json");

    let instrumented_sim = "instrumented.exe";
    // verilog --> sim; adapted from verilator::verilator_build()
    let verilator_out_dir = "verilator-out";
    let sim_bin = `${verilator_out_dir}/Vtoplevel`;
    e.build_cmd(
      [sim_bin],
      "verilator-compile-standalone-tb",
      [instrumented_verilog],
      ["tb.sv"],
    );
    e.arg("out-dir", verilator_out_dir);
    e.build("cp", sim_bin, instrumented_sim);

    let instrumented_vcd = "instrumented.vcd";
    // sim --> vcd; adapted from rtl_sim
    e.build_cmd(
        ["sim.log", instrumented_vcd],
        "sim-run",
        [instrumented_sim, "$datadir"],
        [],
    );
    e.arg("bin", instrumented_sim);
    e.arg("args", `+NOTRACE=0 +OUT=${instrumented_vcd}`);

    // vcd --> flamegraph
    let elems_profiled_json = "elems-profiled.json";
    let flamegraph_folded = "flamegraph.folded";
    e.build_cmd([flamegraph_folded], "parse-vcd", [instrumented_vcd], ["$cells"]);
    e.build_cmd([output], "create-visuals", [flamegraph_folded], []);
}

op(
    "profiler",
    [c::calyx_setup, profiling_setup, v::verilator_setup, sim::sim_setup],
    c::calyx_state,
    flamegraph,
    |e, input, output| calyx_to_flamegraph(e, input, output)
);
