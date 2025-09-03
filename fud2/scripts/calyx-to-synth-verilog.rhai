import "calyx" as c;

export const synth_verilog_state = state("synth-verilog", ["sv"]);

// Helper to compile Calyx to synth-ready SV
fn calyx_to_synth_verilog(e, input, output) {
    // Compile Calyx to synth-ready SV
    e.build_cmd([output], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", "--synthesis -p external --disable-verify");
}

// Op to compile Calyx to synth-ready SV
op(
    "calyx-to-synth-verilog",
    [c::calyx_setup],
    c::calyx_state,
    synth_verilog_state,
    |e, input, output| {
        calyx_to_synth_verilog(e, input, output);
    }
)
