import "calyx" as calyx;

let tb_state = state("tb", []);

fn tb_setup(e) {
    e.config_var("calyx-tb-exe", "tb.exe");
    e.config_var("calyx-tb-test", "tb.test");
    e.config_var("calyx-tb-config-file", "tb.config-file");
    e.config_var_or("calyx-tb-flags", "tb.flags", "");
    e.config_var("calyx-tb-using", "tb.using");
    e.rule("verilog-to-tb", "$calyx-tb-exe $in --test $caller-dir/$calyx-tb-test --using $calyx-tb-using --config $calyx-tb-config-file $calyx-tb-flags");
}

rule([tb_setup], calyx::verilog_state, tb_state, "verilog-to-tb");
