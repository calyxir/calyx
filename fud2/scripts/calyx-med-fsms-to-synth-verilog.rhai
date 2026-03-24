import "calyx" as c;

export const synth_verilog_state = state("med-fsms-synth-verilog", ["sv"]);

// Helper to compile Calyx to synth-ready SV
fn calyx_to_synth_verilog_med_fsms(e, input, output) {
    // Compile Calyx to synth-ready SV
    e.build_cmd([output], "calyx", [input], []);
    e.arg("backend", "verilog");
    e.arg("args", "--synthesis -p synthesis-papercut -p med-fsm -x \
tdcc:infer-fsms -p lower -p externalize --disable-verify"); 
    // same pipeline as -p external but we replace the 'compile' alias in the 
    // pass to med-fsm
}

// Op to compile Calyx to synth-ready SV
op(
    "calyx-med-fsms-to-synth-verilog",
    [c::calyx_setup],
    c::calyx_state,
    synth_verilog_state,
    |e, input, output| {
        calyx_to_synth_verilog_med_fsms(e, input, output);
    }
)
