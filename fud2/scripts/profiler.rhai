import "calyx" as c;
import "verilator" as v;

let profiled = state("profiled", ["json"]);
let folded = state("folded", ["folded"]); // input to flamegraph script
let flamegraph = state("flamegraph", ["svg"]);

// probably want some option for disabling optimizations

fn profiling_setup(e) {
    e.config_var_or("component_cells", "component_cells", "$calyx-base/target/debug/component_cells");
    e.rule("component-cells", "$component_cells -l $calyx-base $in > $out");

    e.config_var_or("component_groups", "component_groups", "$calyx-base/target/debug/component_cells");
    e.rule("component-groups", "$component_groups -l $calyx-base $in > $out");

    // dummy rule to force ninja to build the testbench
    e.rule("dummy", "sh -c 'cat $$0' $in > $out");
}

// operations
// run component-cells and component-groups
// obtain vcd file (probably can obtain tdcc json here?)

fn calyx_to_profiled_helper(e, input, output) {
    let cells = "cells.json";
    let groups = "groups.json";
    let tmp_out = "tmp-out.sv";
    let tdcc_json = "fsm.json"; // might not be necessary if we get rid of fsms?
    e.build_cmd([cells], "component-cells", [input], []);
    e.build_cmd([groups], "component-groups", [input], []);
    e.build_cmd([tmp_out], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", " -p static-inline -p compile-static -p compile-repeat -p par-to-seq -p instrument -p no-opt -x tdcc:dump-fsm-json=fsm.json"); // FIXME: hack?
    e.build_cmd([output], "dummy", [tmp_out, cells, groups], []);
}

op(
    "calyx-to-profiled-verilog",
    [c::calyx_setup, profiling_setup],
    c::calyx_state,
    c::verilog_state,
    |e, input, output| calyx_to_profiled_helper(e, input, output)
);

// running scripts to process vcd file
// run flame graphs script