import "calyx" as c;
import "rtl_sim" as sim;
import "testbench" as tb;

export let icarus_setup = icarus_setup;
fn icarus_setup(e) {
    e.var_("iverilog", "iverilog");
    e.rule(
        "icarus-compile-standalone-tb",
        "$iverilog -g2012 -o $out tb.sv $in",
    );
    e.rule(
        "icarus-compile-custom-tb",
        "$iverilog -g2012 -o $out tb.sv memories.sv $in",
    );
}


op(
    "icarus",
    [sim::sim_setup, tb::standalone_setup, icarus_setup],
    c::verilog_noverify,
    sim::simulator,
    |e, input, output| {
        e.build_cmd(
            [output],
            "icarus-compile-standalone-tb",
            [input],
            ["tb.sv"],
        );
    },
);

op(
    "icarus-refmem",
    [sim::sim_setup, icarus_setup],
    tb::verilog_refmem_noverify,
    sim::simulator,
    |e, input, output| {
        e.build_cmd(
            [output],
            "icarus-compile-custom-tb",
            [input],
            ["tb.sv", "memories.sv"],
        );
    },
);
