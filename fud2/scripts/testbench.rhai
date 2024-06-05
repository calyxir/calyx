// Defines testbenches
export const verilog_refmem = state("verilog-refmem", ["sv"]);
export const verilog_refmem_noverify = state("verilog-refmem-noverify", ["sv"]);

// [Default] Setup for using rsrc/tb.sv as testbench (and managing memories within the design)
export const standalone_setup = standalone_testbench_setup;
fn standalone_testbench_setup(e) {
    // Standalone Verilog testbench.
    e.rsrc("tb.sv");
}

// [Needs YXI backend compiled] Setup for creating a custom testbench (needed for FIRRTL)
export const custom_setup = custom_testbench_setup;
fn custom_testbench_setup(e) {
    // Convert all ref cells to @external (FIXME: YXI should work for both?)
    e.rule("ref-to-external", "sed 's/ref /@external /g' $in > $out");

    // Convert all @external cells to ref (FIXME: we want to deprecate @external)
    e.rule("external-to-ref", "sed 's/@external([0-9]*)/ref/g' $in | sed 's/@external/ref/g' > $out");

    e.var_(
        "gen-testbench-script",
        "$calyx-base/tools/firrtl/generate-testbench.py",
    );
    e.rsrc("memories.sv"); // Memory primitives.

    e.rule(
        "generate-refmem-testbench",
        "python3 $gen-testbench-script $in > $out",
    );

    // dummy rule to force ninja to build the testbench
    e.rule("dummy", "sh -c 'cat $$0' $in > $out");
};
