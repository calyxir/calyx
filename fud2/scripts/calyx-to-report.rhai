import "calyx" as c;
import "profiler" as p;
import "calyx-to-synth-verilog" as csv;

export const utilization_report_state = state("utilization-report", ["rpt"]);
export const timing_report_state = state("timing-report", ["rpt"]);
export const area_report_state = state("area-report", ["rpt"]);
export const json_report_state = state("json-report", ["json"]);

export const synth_setup = synth_setup;

fn synth_setup(e) {
    // We need the Calyx directory root to access scripts
    e.config_var("calyx-base", "calyx.base");

    // Some base rules for file manipulation
    e.rule("mkdir", "mkdir -p $out");
    e.rule("move", "mv $in $out");
    e.rule("copy", "cp $in $out");

    // Specific rules to copy out each report
    e.rule("copy-utilization", "cp out/FutilBuild.runs/impl_1/main_utilization_placed.rpt $out");
    e.rule("copy-timing", "cp out/FutilBuild.runs/impl_1/main_timing_summary_routed.rpt $out");
    e.rule("copy-area", "cp out/hierarchical_utilization_placed.rpt $out");

    // Rule for Vivado run, we suppress output since it gets written to a log file anyway
    e.rule("vivado", "vivado -mode batch -source synth.tcl > /dev/null");

    // Read config variable for the device.xdc file, use default one otherwise
    e.config_var_or("device_xdc", "synth-verilog.constraints", "default.xdc");
    e.rsrc("default.xdc");

    // Use default synth.tcl file
    e.rsrc("synth.tcl");

    // Python scripts for parsing reports for visualization and extracting JSON summary
    e.rule("parse-rpt", "synthrep viz -t flamegraph -f $in > $out");
    e.rule("extract-rpt", "synthrep summary > $out");

    // Bash script to create a FlameGraph
    e.config_var("flamegraph-script", "flamegraph.script");
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.sh");
    e.rule("create-visuals", "bash $create-visuals-script $flamegraph-script . $in $out");
}

// Helper to compile Calyx to SV and run Vivado on it
fn synth_verilog_through_vivado(e, input, output) {
    // Copy over XDC file
    e.build_cmd(["device.xdc"], "copy", ["$device_xdc"], []);
    // IGNOREME is needed by Ninja to consider this command as a build target
    e.build_cmd(["IGNOREME"], "vivado", [], ["main.sv", "synth.tcl", "device.xdc"]);
}

// Helper to chain Calyx-SV compilation, synthesis, and report extraction
fn synth_verilog_to_report(e, input, output, rtype) {
    synth_verilog_through_vivado(e, input, output);
    // Similarly IGNOREME is needed here as a dependency so that Ninja threads this command
    // with the Vivado synthesis command
    e.build_cmd([output], `copy-${rtype}`, ["IGNOREME"], []);
}

// Op to get utilization report from synth-ready SV
op(
    "synth-verilog-to-utilization-report",
    [synth_setup],
    csv::synth_verilog_state,
    utilization_report_state,
    |e, input, output| {
        synth_verilog_to_report(e, input, output, "utilization");
    }
);

// Op to get timing report from synth-ready SV
op(
    "synth-verilog-to-timing-report",
    [synth_setup],
    csv::synth_verilog_state,
    timing_report_state,
    |e, input, output| {
        synth_verilog_to_report(e, input, output, "timing");
    }
);

// Op to get area report from synth-ready SV
op(
    "synth-verilog-to-area-report",
    [synth_setup],
    csv::synth_verilog_state,
    area_report_state,
    |e, input, output| {
        synth_verilog_to_report(e, input, output, "area");
    }
);

// Op to get synthesis JSON summary from synth-ready SV
op(
    "synth-verilog-to-synthesis-json",
    [synth_setup],
    csv::synth_verilog_state,
    json_report_state,
    |e, input, output| {
        synth_verilog_through_vivado(e, input, output);
        // Similarly IGNOREME is needed here as a dependency so that Ninja threads this command
        // with the Vivado synthesis command
        e.build_cmd([output], "extract-rpt", ["IGNOREME"], []);
    }
);

// Op to get FlameGraph from area report
op(
    "area-report-to-flamegraph",
    [synth_setup],
    area_report_state,
    p::flamegraph,
    |e, input, output| {
        let folded = "flamegraph.folded";
        e.build_cmd([folded], "parse-rpt", [input], []);
        e.build_cmd([output], "create-visuals", [folded], []);
    }
);
