import "calyx" as c;
import "profiler" as p;

export const utilization_report_state = state("utilization-report", ["urpt"]);
export const timing_report_state = state("timing-report", ["trpt"]);
export const area_report_state = state("area-report", ["arpt"]);
export const json_report_state = state("json-report", ["rjson"]);

export const synth_setup = synth_setup;

fn synth_setup(e) {
    // We need Calyx for compilation to SV
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
    e.var_("parse-rpt-script", "$calyx-base/tools/report-parsing/rpt.py");
    e.rule("parse-rpt", "python3 $parse-rpt-script $in > $out");
    e.var_("extract-rpt-script", "$calyx-base/tools/report-parsing/extract.py");
    e.rule("extract-rpt", "python3 $extract-rpt-script > $out");

    // Bash script to create a FlameGraph
    e.config_var("flamegraph-script", "flamegraph.script");
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.sh");
    e.rule("create-visuals", "bash $create-visuals-script $flamegraph-script . $in $out");
}

// Helper to compile Calyx to SV and run Vivado on it
fn calyx_through_vivado(e, input, output) {
    // Compile Calyx to SV
    e.build_cmd(["main.sv"], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", "--synthesis -p external --disable-verify");
    // Copy over XDC file and run synthesis
    e.build_cmd(["device.xdc"], "copy", ["$device_xdc"], []);
    // IGNOREME is needed by Ninja to consider this command as a build target
    e.build_cmd(["IGNOREME"], "vivado", [], ["main.sv", "synth.tcl", "device.xdc"]);
}

// Helper to chain Calyx-SV compilation, synthesis, and report extraction
fn calyx_to_report(e, input, output, rtype) {
    calyx_through_vivado(e, input, output);
    // Similarly IGNOREME is needed here as a dependency so that Ninja threads this command
    // with the Vivado synthesis command
    e.build_cmd([output], `copy-${rtype}`, ["IGNOREME"], []);
}

// Op to get utilization report from Calyx
op(
    "calyx-to-utilization-report",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    utilization_report_state,
    |e, input, output| {
        calyx_to_report(e, input, output, "utilization");
    }
);

// Op to get timing report from Calyx
op(
    "calyx-to-timing-report",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    timing_report_state,
    |e, input, output| {
        calyx_to_report(e, input, output, "timing");
    }
);

// Op to get area report from Calyx
op(
    "calyx-to-area-report",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    area_report_state,
    |e, input, output| {
        calyx_to_report(e, input, output, "area");
    }
);

// Op to get synthesis JSON summary from Calyx
op(
    "calyx-to-synthesis-json",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    json_report_state,
    |e, input, output| {
        calyx_through_vivado(e, input, output);
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
