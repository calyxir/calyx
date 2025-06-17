import "calyx" as c;
import "profiler" as p;

export const flamegraph_folded_state = state("flamegraph-folded", ["folded"]);
export const utilization_report_state = state("utilization-report", ["urpt"]);
export const timing_report_state = state("timing-report", ["trpt"]);
export const area_report_state = state("area-report", ["arpt"]);

export const synth_setup = synth_setup;

fn synth_setup(e) {
    e.config_var("calyx-base", "calyx.base");
    e.rule("mkdir", "mkdir -p $out");
    e.rule("move", "mv $in $out");
    e.rule("copy-utilization", "cp out/FutilBuild.runs/impl_1/main_utilization_placed.rpt $out");
    e.rule("copy-timing", "cp out/FutilBuild.runs/impl_1/main_timing_summary_routed.rpt $out");
    e.rule("copy-area", "cp out/hierarchical_utilization_placed.rpt $out");
    e.rule("vivado", "vivado -mode batch -source synth.tcl");
    e.rsrc("synth.tcl");
    e.rsrc("device.xdc");
    e.var_("parse-rpt-script", "$calyx-base/tools/area-profiler/rpt.py");
    e.rule("parse-rpt", "python3 $parse-rpt-script $in > $out");
    e.config_var("flamegraph-script", "flamegraph.script");
    e.var_("create-visuals-script", "$calyx-base/tools/profiler/create-visuals.sh");
    e.rule("create-visuals", "bash $create-visuals-script $flamegraph-script . $in $out");
}

op(
    "calyx-to-utilization-report",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    utilization_report_state,
    |e, input, output| {
        e.build_cmd(["main.sv"], "calyx", [input], []);
        e.arg("backend", "verilog");
        e.arg("args", "--synthesis -p external --disable-verify");
        e.build_cmd(["IGNOREME.txt"], "vivado", [], ["main.sv", "synth.tcl", "device.xdc"]);
        e.build_cmd([output], "copy-utilization", ["IGNOREME.txt"], []);
    }
);

op(
    "calyx-to-timing-report",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    timing_report_state,
    |e, input, output| {
        e.build_cmd(["main.sv"], "calyx", [input], []);
        e.arg("backend", "verilog");
        e.arg("args", "--synthesis -p external --disable-verify");
        e.build_cmd(["IGNOREME.txt"], "vivado", [], ["main.sv", "synth.tcl", "device.xdc"]);
        e.build_cmd([output], "copy-timing", ["IGNOREME.txt"], []);
    }
);

op(
    "calyx-to-area-report",
    [synth_setup, c::calyx_setup],
    c::calyx_state,
    area_report_state,
    |e, input, output| {
        e.build_cmd(["main.sv"], "calyx", [input], []);
        e.arg("backend", "verilog");
        e.arg("args", "--synthesis -p external --disable-verify");
        e.build_cmd(["IGNOREME.txt"], "vivado", [], ["main.sv", "synth.tcl", "device.xdc"]);
        e.build_cmd([output], "copy-area", ["IGNOREME.txt"], []);
    }
);

op(
    "area-report-to-flamegraph-folded",
    [synth_setup],
    area_report_state,
    flamegraph_folded_state,
    |e, input, output| {
       e.build_cmd([output], "parse-rpt", [input], []);
    }
);

op(
    "flamegraph-folded-to-flamegraph",
    [synth_setup, c::calyx_setup],
    flamegraph_folded_state,
    p::flamegraph,
    |e, input, output| {
       e.build_cmd([output], "create-visuals", [input], []);
    }
);
