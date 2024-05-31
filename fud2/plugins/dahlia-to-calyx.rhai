let dahlia = state("dahlia", ["fuse"]);

fn dahlia_setup(e) {
   e.config_var("dahlia-exe", "dahlia");
   e.rule("dahlia-to-calyx", "$dahlia-exe -b calyx --lower -l error $in -o $out");
}

rule([dahlia_setup], dahlia, get_state("calyx"), "dahlia-to-calyx");
