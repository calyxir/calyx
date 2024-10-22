import "calyx" as c;
import "verilator" as v;
import "rtl_sim" as sim;

let profiled = state("profiled", ["json"]);
let folded = state("folded", ["folded"]); // input to flamegraph script
let flamegraph = state("flamegraph", ["svg"]);

// probably want some option for disabling optimizations

fn profiling_setup(e) {
    e.var_("cells", "cells.json");
    e.var_("groups", "groups.json");
    e.var_("tdcc-json", "fsm.json"); // might not be necessary if we get rid of fsms?

    // rules for preprocessing

    e.config_var_or("component_cells", "component_cells", "$calyx-base/target/debug/component_cells");
    e.rule("component-cells", "$component_cells -l $calyx-base $in > $out");

    e.config_var_or("component_groups", "component_groups", "$calyx-base/target/debug/component_groups");
    e.rule("component-groups", "$component_groups -l $calyx-base $in > $out");

    // dummy rule to force ninja to build the testbench
    e.rule("dummy", "sh -c 'cat $$0' $in > $out");

    // rules for postprocessing

    // FIXME: need better names
    // script to process vcd and attribute active cycles to every group/cell
    e.var_("parse-vcd-script", "$calyx-base/tools/profiler/parse-vcd.py");
    // FIXME: probably broken
    e.rule("parse-vcd", "python3 $parse-vcd-script $in $tdcc-json $cells $groups summary.csv $out");

    // script to produce trace and visuals
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.py");
    // FIXME: probably broken
    e.rule("create-visuals", "python3 $create-visuals-script $in $cells timeline.json fsm-timeline.json $out fsm-flame.folded frequency.folded components.folded fsm-components.folded");

    e.rsrc("flamegraph.pl"); // flame graph script
    e.rule("produce-flame-graph", "./flamegraph.pl $in > $out");
}

fn calyx_to_profiled_helper(e, input, output) { // I want to add an extra argument deciding whether we're optimizing the program or not
    let tmp_out = "tmp-out.sv";
    e.build_cmd(["$cells"], "component-cells", [input], []);
    e.build_cmd(["$groups"], "component-groups", [input], []);
    e.build_cmd([tmp_out], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", " -p static-inline -p compile-static -p compile-repeat -p par-to-seq -p instrument -p no-opt -x tdcc:dump-fsm-json=fsm.json"); // FIXME: hack?
    e.build_cmd([output], "dummy", [tmp_out, "$cells", "$groups"], []);
}

op(
    "calyx-to-profiled-verilog",
    [c::calyx_setup, profiling_setup],
    c::calyx_state,
    c::verilog_state,
    |e, input, output| calyx_to_profiled_helper(e, input, output)
);

// running scripts to process vcd file
op(
    "trace-to-profiled",
    [profiling_setup],
    sim::vcd,
    flamegraph,
    |e, input, output| {
        let elems_profiled_json = "elems-profiled.json";
        let flamegraph_folded = "flamegraph.folded";
        e.build_cmd([elems_profiled_json], "parse-vcd", [input], []);
        e.build_cmd([flamegraph_folded], "create-visuals", [elems_profiled_json], []);
        e.build_cmd([output], "produce-flame-graph", [flamegraph_folded], ["flamegraph.pl"]);
    }
);
// run flame graphs script