import "calyx" as c;
import "calyx_py-to-calyx" as cp;
import "verilator" as v;
import "rtl_sim" as sim;

export const flamegraph = state("flamegraph", ["svg"]);

fn profiling_setup(e) {
    e.var_("cells", "cells.json");
    e.var_("fsm-json", "fsm.json");
    e.var_("cell-share-json", "shared-cells.json");
    // series of passes after validation and instrumentation
    e.config_var_or("passes", "profiler.compilation-passes", "-p pre-opt -p compile -p post-opt -p lower"); // set passes="-p compile-sync -p simplify-with-control -p compile-invoke -p compile -p lower" to run without optimizations

    // eDSL python file arguments
    e.config_var_or("edsl-args", "profiler.edsl-args", "");

    // rules for preprocessing

    e.config_var_or("component_cells", "component_cells", "$calyx-base/target/debug/component_cells");
    e.rule("component-cells", "$component_cells -l $calyx-base $in > $out");

    // rules for postprocessing

    // script to process vcd and produce folded representation of flame graph
    e.var_("parse-vcd-script", "$calyx-base/tools/profiler/profiler-process/main.py");
    e.rule("parse-vcd", "python3 $parse-vcd-script $in $cells $fsm-json $cell-share-json profiler-out $out --print-trace-threshold 100");

    e.config_var("flamegraph-script", "flamegraph.script");
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.sh");
    e.rule("create-visuals", "bash $create-visuals-script $flamegraph-script profiler-out $in $out");

    // Standalone Verilog testbench. copied from testbench
    e.rsrc("tb.sv");

    // ---- eDSL to Calyx to flame graph.
    e.rule("edsl-to-calyx", "python3 $in $edsl-args > $out");

    e.var_("metadata-mapping-json", "metadata-map.json");
    e.config_var_or("fileinfo_emitter", "fileinfo_emitter", "$calyx-base/target/debug/fileinfo_emitter");
    e.rule("parse-metadata", "$fileinfo_emitter -l $calyx-base $in > $metadata-mapping-json");

    e.rule("parse-vcd-from-adl", "python3 $parse-vcd-script $in $cells $fsm-json $cell-share-json profiler-out $out --adl-mapping-file $metadata-mapping-json --print-trace-threshold 100");
}

// TODO: merge with calyx_to_flamegraph
fn edsl_to_flamegraph(e, input, output) {
    // create calyx file
    let calyx = "calyx.futil";
    e.build_cmd([calyx], "edsl-to-calyx", [input], []);

    // create metatdata mapping file
    e.build_cmd(["$metadata-mapping-json"], "parse-metadata", [calyx], []);

    // instrument calyx and produce verilog
    let instrumented_verilog = "instrumented.sv";
    e.build_cmd(["$cells"], "component-cells", [calyx], []);
    e.build_cmd([instrumented_verilog], "calyx", [calyx], []);
    e.arg("backend", "verilog");
    e.arg("args", "-p validate -p group2seq -p compile-invoke -p profiler-instrumentation $passes -x tdcc:dump-fsm-json=fsm.json -x cell-share:emit-share-map=shared-cells.json");

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
    e.build_cmd([flamegraph_folded], "parse-vcd-from-adl", [instrumented_vcd], ["$cells", "$metadata-mapping-json"]);
    e.build_cmd([output], "create-visuals", [flamegraph_folded], []);
}

fn calyx_to_flamegraph(e, input, output) {
    // instrument calyx and produce verilog
    let instrumented_verilog = "instrumented.sv";
    e.build_cmd(["$cells"], "component-cells", [input], []);
    e.build_cmd([instrumented_verilog], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", "-p validate -p group2seq -p compile-invoke -p profiler-instrumentation $passes -x tdcc:dump-fsm-json=fsm.json -x cell-share:emit-share-map=shared-cells.json");

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

op("calyx-py-profiler",
    [c::calyx_setup, profiling_setup, v::verilator_setup, sim::sim_setup],
    cp::calyx_py,
    flamegraph,
    |e, input, output| edsl_to_flamegraph(e, input, output)
)