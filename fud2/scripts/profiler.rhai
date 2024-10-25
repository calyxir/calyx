import "calyx" as c;

export const instrumented_verilog = state("verilog-instrumented", ["sv"]);
export const instrumented_sim = state("sim-instrumented", ["exe"]);
export const instrumented_vcd = state("vcd-instrumented", ["vcd"]);
export const flamegraph = state("flamegraph", ["svg"]);

fn profiling_setup(e) {
    e.var_("cells", "cells.json");
    e.var_("tdcc-json", "fsm.json"); // might not be necessary if we get rid of fsms?

    // series of passes after instrumentation?
    e.config_var_or("passes", "passes", "all"); // set passes="no-opt" to run without optimizations

    // rules for preprocessing

    e.config_var_or("component_cells", "component_cells", "$calyx-base/target/debug/component_cells");
    e.rule("component-cells", "$component_cells -l $calyx-base $in > $out");

    // dummy rule to force ninja to build the testbench
    e.rule("dummy", "sh -c 'cat $$0' $in > $out");

    // rules for postprocessing

    // script to process vcd and attribute active cycles to every group/cell
    e.var_("parse-vcd-script", "$calyx-base/tools/profiler/parse-vcd.py");
    e.rule("parse-vcd", "python3 $parse-vcd-script $in $tdcc-json $cells summary.csv $out");

    // script to produce trace and visuals
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.py");
    e.rule("create-visuals", "python3 $create-visuals-script $in $cells timeline.json fsm-timeline.json $out fsm-flame.folded frequency.folded components.folded fsm-components.folded");

    e.config_var("flamegraph-script", "flamegraph.script");
    e.rule("produce-flame-graph", "$flamegraph-script $in > $out");

}

fn calyx_to_profiled_helper(e, input, output) { // I want to add an extra argument deciding whether we're optimizing the program or not
    let tmp_out = "tmp-out.sv";
    e.build_cmd(["$cells"], "component-cells", [input], []);
    e.build_cmd([tmp_out], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", " -p static-inline -p compile-static -p compile-repeat -p par-to-seq -p profiler-instrumentation -p $passes -x tdcc:dump-fsm-json=fsm.json");
    e.build_cmd([output], "dummy", [tmp_out, "$cells"], []);
}

op(
    "calyx-to-instrumented-verilog",
    [c::calyx_setup, profiling_setup],
    c::calyx_state,
    instrumented_verilog,
    |e, input, output| calyx_to_profiled_helper(e, input, output)
);

op(
    "instrumented-trace-to-profiled",
    [profiling_setup],
    instrumented_vcd,
    flamegraph,
    |e, input, output| {
        let elems_profiled_json = "elems-profiled.json";
        let flamegraph_folded = "flamegraph.folded";
        e.build_cmd([elems_profiled_json], "parse-vcd", [input], []);
        e.build_cmd([flamegraph_folded], "create-visuals", [elems_profiled_json], []);
        e.build_cmd([output], "produce-flame-graph", [flamegraph_folded], []);
    }
);