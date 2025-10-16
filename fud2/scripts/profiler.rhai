import "calyx" as c;
import "calyx_py-to-calyx" as cp;
import "dahlia-to-calyx" as d;
import "verilator" as v;
import "rtl_sim" as sim;

export const flamegraph = state("flamegraph", ["svg"]);

fn profiling_setup(e) {
    // files produced via compiler + tools that will be used in postprocessing
    e.var_("cell-json", "cells.json");
    e.var_("fsm-json", "fsm.json");
    e.var_("cell-share-json", "shared-cells.json");
    e.var_("enable-par-track-json", "enable-par-track.json");
    e.var_("path-descriptors-json", "path-descriptors.json");
    /// Control group position (from Calyx file), line number, and type of control node.
    e.var_("ctrl-pos-json", "ctrl-pos.json");
    /// ADL source mapping.
    e.var_("adl-metadata-json", "adl-metadata.json");

    // series of passes after validation and instrumentation
    e.config_var_or("passes", "profiler.compilation-passes", "-p pre-opt -p compile -p post-opt -p lower"); // set passes="-p compile-sync -p simplify-with-control -p compile-invoke -p compile -p lower" to run without optimizations

    // eDSL python file arguments
    e.config_var_or("edsl-args", "profiler.edsl-args", "");

    // rules for preprocessing

    e.config_var_or("component_cells", "component_cells", "$calyx-base/target/debug/component_cells");
    e.rule("component-cells", "$component_cells -l $calyx-base $in > $out");

    // rules for postprocessing

    e.config_var("flamegraph-script", "flamegraph.script");
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.sh");
    e.rule("create-visuals", "bash $create-visuals-script $flamegraph-script profiler-out $in $out");

    // Standalone Verilog testbench. copied from testbench
    e.rsrc("tb.sv");

    // ---- eDSL to Calyx to flame graph.
    e.rule("edsl-to-calyx", "python3 $in $edsl-args > $out");

    e.var_("metadata-mapping-json", "metadata-map.json");
    e.config_var_or("fileinfo_emitter", "fileinfo_emitter", "$calyx-base/target/debug/fileinfo_emitter");
    // e.rule("parse-metadata", "$fileinfo_emitter -l $calyx-base $in > $metadata-mapping-json");
    e.rule("parse-metadata", "$fileinfo_emitter -l $calyx-base $in -c $ctrl-pos-json");
    e.rule("parse-metadata-with-adl", "$fileinfo_emitter -l $calyx-base $in -c $ctrl-pos-json -o $adl-metadata-json");
}

fn profiler_non_synth_setup(e) {
    // script to process vcd and produce folded representation of flame graph
    e.rule("parse-vcd", "profiler $in $cell-json $fsm-json $cell-share-json $enable-par-track-json $path-descriptors-json profiler-out $out --print-trace-threshold 100 --ctrl-pos-file $ctrl-pos-json");
    e.rule("parse-vcd-from-adl", "profiler $in $cell-json $fsm-json $cell-share-json $enable-par-track-json $path-descriptors-json profiler-out $out --ctrl-pos-file $ctrl-pos-json --adl-mapping-file $adl-metadata-json --print-trace-threshold 100");
    e.rule("parse-vcd-from-dahlia", "profiler $in $cell-json $fsm-json $cell-share-json $enable-par-track-json $path-descriptors-json profiler-out $out --ctrl-pos-file $ctrl-pos-json --dahlia-parent-map $parent-map --adl-mapping-file $adl-metadata-json --print-trace-threshold 100");
}

fn profiler_synth_setup(e) {
    // location of the hierarchy json file from synthesis
    e.config_var("hierarchy", "synth-verilog.hier");
    // synthesis variable to plot, one of: ff (FlipFlops), lut (Look-Up Tables), llut (Logical Look-Up Tables), lutram (Look-Up Table RAMs)
    e.config_var("variable", "synth-verilog.var");
    // script to process vcd and produce folded representation of flame graph as well as html plots
    e.rule("parse-vcd", "profiler $in $cell-json $fsm-json $cell-share-json $enable-par-track-json $path-descriptors-json profiler-out $out --print-trace-threshold 100 --utilization-report-json $hierarchy --utilization-variable $variable");
}

// TODO: merge with calyx_to_flamegraph
fn edsl_to_flamegraph(e, input, output) {
    // create calyx file
    let calyx = "calyx.futil";
    e.build_cmd([calyx], "edsl-to-calyx", [input], []);

    let metadata_calyx = "metadata-calyx.futil";
    e.build_cmd([metadata_calyx], "calyx-pass", [calyx], []);
    e.arg("pass", "metadata-table-generation");

    // create metatdata mapping file
    e.build_cmd(["$ctrl-pos-json", "$adl-metadata-json"], "parse-metadata-with-adl", [metadata_calyx], []);

    // instrument calyx and produce verilog
    let instrumented_verilog = "instrumented.sv";
    e.build_cmd(["$cell-json"], "component-cells", [calyx], []);
    e.build_cmd([instrumented_verilog], "calyx", [calyx], []);
    e.arg("backend", "verilog");
    // NOTE: disabling group2seq because it may cause cycle count differences btwn profiled vs non-profiled versions.
    e.arg("args", "-d group2seq -p metadata-table-generation -p validate -p compile-invoke -p uniquefy-enables -p profiler-instrumentation -p dead-group-removal $passes -x tdcc:dump-fsm-json=fsm.json -x cell-share:emit-share-map=shared-cells.json -x uniquefy-enables:par-thread-json=enable-par-track.json -x uniquefy-enables:path-descriptor-json=path-descriptors.json");

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
    e.build_cmd([flamegraph_folded], "parse-vcd-from-adl", [instrumented_vcd], ["$cell-json", "$adl-metadata-json"]);
    e.build_cmd([output], "create-visuals", [flamegraph_folded], []);
}

fn dahlia_to_profiled(e, input, output) {
    // compile Dahlia with extra args to give us the hierarchy
    let calyx = "calyx.futil";
    e.build_cmd([calyx], "dahlia-to-calyx", [input], []);
    e.arg("dahlia-args", "--parent-map $parent-map");

    let metadata_calyx = "metadata-calyx.futil";
    e.build_cmd([metadata_calyx], "calyx-pass", [calyx], []);
    e.arg("pass", "metadata-table-generation");

    // create metatdata mapping file
    e.build_cmd(["$ctrl-pos-json", "$adl-metadata-json"], "parse-metadata-with-adl", [metadata_calyx], []);

    // instrument calyx and produce verilog
    let instrumented_verilog = "instrumented.sv";
    e.build_cmd(["$cell-json"], "component-cells", [calyx], []);
    e.build_cmd([instrumented_verilog], "calyx", [calyx], []);
    e.arg("backend", "verilog");
    // NOTE: disabling group2seq because it may cause cycle count differences btwn profiled vs non-profiled versions.
    e.arg("args", "-d group2seq -p metadata-table-generation -p validate -p compile-invoke -p uniquefy-enables -p profiler-instrumentation -p dead-group-removal $passes -x tdcc:dump-fsm-json=fsm.json -x cell-share:emit-share-map=shared-cells.json -x uniquefy-enables:par-thread-json=enable-par-track.json -x uniquefy-enables:path-descriptor-json=path-descriptors.json");

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
    e.build_cmd([flamegraph_folded], "parse-vcd-from-dahlia", [instrumented_vcd], ["$cell-json", "$adl-metadata-json"]);
    e.build_cmd([output], "create-visuals", [flamegraph_folded], []);
}

fn calyx_to_flamegraph(e, input, output) {
    // First pass: obtain control metadata information by running metadata-table-generation
    // on the original program and then running the metadata tool on it.
    let metadata_calyx = "metadata-calyx.futil";
    e.build_cmd([metadata_calyx], "calyx-pass", [input], []);
    e.arg("pass", "metadata-table-generation");
    
    // generate ctrl-pos-json by running the metadata emitter on it
    e.build_cmd(["$ctrl-pos-json"], "parse-metadata", [metadata_calyx], []);

    // instrument calyx and produce verilog
    let instrumented_verilog = "instrumented.sv";
    e.build_cmd(["$cell-json"], "component-cells", [input], []);
    e.build_cmd([instrumented_verilog], "calyx", [input], []);
    e.arg("backend", "verilog");
    // NOTE: disabling group2seq because it may cause cycle count differences btwn profiled vs non-profiled versions.
    e.arg("args", "-d group2seq -p metadata-table-generation -p validate -p compile-invoke -p uniquefy-enables -p profiler-instrumentation -p dead-group-removal $passes -x tdcc:dump-fsm-json=fsm.json -x cell-share:emit-share-map=shared-cells.json -x uniquefy-enables:par-thread-json=enable-par-track.json -x uniquefy-enables:path-descriptor-json=path-descriptors.json");

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
    e.build_cmd([flamegraph_folded], "parse-vcd", [instrumented_vcd], ["$cell-json", "$ctrl-pos-json"]);
    e.build_cmd([output], "create-visuals", [flamegraph_folded], []);
}

op(
    "profiler",
    [c::calyx_setup, profiling_setup, profiler_non_synth_setup, v::verilator_setup, sim::sim_setup],
    c::calyx_state,
    flamegraph,
    |e, input, output| calyx_to_flamegraph(e, input, output)
);

op(
    "profiler-area-util",
    [c::calyx_setup, profiling_setup, profiler_synth_setup, v::verilator_setup, sim::sim_setup],
    c::calyx_state,
    flamegraph,
    |e, input, output| calyx_to_flamegraph(e, input, output)
);

op("calyx-py-profiler",
    [c::calyx_setup, profiling_setup, profiler_non_synth_setup, v::verilator_setup, sim::sim_setup],
    cp::calyx_py,
    flamegraph,
    |e, input, output| edsl_to_flamegraph(e, input, output)
);

op("dahlia-profiler",
   [d::dahlia_setup, c::calyx_setup, profiling_setup, profiler_non_synth_setup, v::verilator_setup, sim::sim_setup],
   d::dahlia,
    flamegraph,
    |e, input, output| dahlia_to_profiled(e, input, output)
)