import "calyx" as c;
import "testbench" as tb;

// setup for FIRRTL-implemented primitives
fn firrtl_primitives_setup(e) {
    // Produce FIRRTL with FIRRTL-defined primitives.
    e.var_(
        "gen-firrtl-primitives-script",
        "$calyx-base/tools/firrtl/generate-firrtl-with-primitives.py",
    );
    e.rule(
        "generate-firrtl-with-primitives",
        "python3 $gen-firrtl-primitives-script $in > $out",
    );
}

fn calyx_to_firrtl_helper(e, input, output, firrtl_primitives) {
    // Temporary Calyx where all refs are converted into external (FIXME: fix YXI to emit for ref as well?)
    let only_externals_calyx = "external.futil";
    // Temporary Calyx where all externals are converted into refs (for FIRRTL backend)
    let only_refs_calyx = "ref.futil";
    // JSON with memory information created by YXI
    let memories_json = "memory-info.json";
    // Custom testbench (same name as standalone testbench)
    let testbench = "tb.sv";
    // Holds contents of file we want to output. Gets cat-ed via final dummy command
    let tmp_out = "tmp-out.fir";
    // Convert ref into external to get YXI working (FIXME: fix YXI to emit for ref as well?)
    e.build_cmd([only_externals_calyx], "ref-to-external", [input], []);
    // Convert external to ref to get FIRRTL backend working
    e.build_cmd([only_refs_calyx], "external-to-ref", [input], []);

    // Get YXI to generate JSON for testbench generation
    e.build_cmd([memories_json], "yxi", [only_externals_calyx], []);
    // generate custom testbench
    e.build_cmd(
        [testbench],
        "generate-refmem-testbench",
        [memories_json],
        [],
    );

    if firrtl_primitives {
        let core_program_firrtl = "core.fir";

        // Obtain FIRRTL of core program
        e.build_cmd(
            [core_program_firrtl],
            "calyx",
            [only_refs_calyx],
            [],
        );
        e.arg("backend", "firrtl");
        e.arg("args", "--synthesis");

        // Obtain primitive uses JSON for metaprogramming
        let primitive_uses_json = "primitive-uses.json";
        e.build_cmd(
            [primitive_uses_json],
            "calyx",
            [only_refs_calyx],
            [],
        );
        e.arg("backend", "primitive-uses");
        e.arg("args", "--synthesis");

        // run metaprogramming script to get FIRRTL with primitives
        e.build_cmd(
            [tmp_out],
            "generate-firrtl-with-primitives",
            [core_program_firrtl, primitive_uses_json],
            [],
        );
    } else {
        // emit extmodule declarations to use Verilog primitive implementations
        e.build_cmd([tmp_out], "calyx", [only_refs_calyx], []);
        e.arg("backend", "firrtl");
        e.arg("args", "--emit-primitive-extmodules");
    }

    // dummy command to make sure custom testbench is created but not emitted as final answer
    e.build_cmd([output], "dummy", [tmp_out, testbench], []);
}

// Calyx to FIRRTL.
let firrtl = state("firrtl", ["fir"]); // using Verilog primitives
let firrtl_with_primitives = state("firrtl-with-primitives", ["fir"]); // using FIRRTL primitives

// let calyx = get_state("calyx");
// let calyx_setup = get_setup("Calyx compiler");
// let custom_testbench_setup = get_setup("Custom Testbench Setup");

op(
    // use Verilog
    "calyx-to-firrtl",
    [c::calyx_setup, tb::custom_setup],
    c::calyx_state,
    firrtl,
    |e, input, output| calyx_to_firrtl_helper(e, input, output, false),
);

op(
    "firrtl-with-primitives",
    [c::calyx_setup, firrtl_primitives_setup, tb::custom_setup],
    c::calyx_state,
    firrtl_with_primitives,
    |e, input, output| calyx_to_firrtl_helper(e, input, output, true),
);

// The FIRRTL compiler.
fn firrtl_setup(e) {
    e.config_var("firrtl-exe", "firrtl.exe");
    e.rule("firrtl", "$firrtl-exe -i $in -o $out -X sverilog");

    e.rsrc("primitives-for-firrtl.sv");
    // adding Verilog implementations of primitives to FIRRTL --> Verilog compiled code
    e.rule(
        "add-verilog-primitives",
        "cat primitives-for-firrtl.sv $in > $out",
    );
}

fn firrtl_compile_helper(e, input, output, firrtl_primitives) {
    if firrtl_primitives {
        e.build_cmd([output], "firrtl", [input], []);
    } else {
        let tmp_verilog = "partial.sv";
        e.build_cmd([tmp_verilog], "firrtl", [input], []);
        e.build_cmd(
            [output],
            "add-verilog-primitives",
            [tmp_verilog],
            ["primitives-for-firrtl.sv"],
        );
    }
}

// FIRRTL --> Verilog compilation using Verilog primitive implementations for Verilator
op(
    "firrtl",
    [firrtl_setup],
    firrtl,
    tb::verilog_refmem,
    |e, input, output| firrtl_compile_helper(e, input, output, false),
);

// FIRRTL --> Verilog compilation using Verilog primitive implementations for Icarus
// This is a bit of a hack, but the Icarus-friendly "noverify" state is identical for this path
// (since FIRRTL compilation doesn't come with verification).
op(
    "firrtl-noverify",
    [firrtl_setup],
    firrtl,
    tb::verilog_refmem_noverify,
    |e, input, output| firrtl_compile_helper(e, input, output, false),
);

// FIRRTL --> Verilog compilation using FIRRTL primitive implementations for Verilator
op(
    "firrtl-with-primitives",
    [firrtl_setup],
    firrtl_with_primitives,
    tb::verilog_refmem,
    |e, input, output| firrtl_compile_helper(e, input, output, true),
);

// FIRRTL --> Verilog compilation using FIRRTL primitive implementations for Icarus
op(
    "firrtl-with-primitives-noverify",
    [firrtl_setup],
    firrtl_with_primitives,
    tb::verilog_refmem_noverify,
    |e, input, output| firrtl_compile_helper(e, input, output, true),
);
