import "calyx" as c;

export const dahlia = state("dahlia", ["fuse"]);

fn dahlia_setup(e) {
   e.config_var("dahlia-exe", "dahlia");
   e.rule("dahlia-to-calyx", "$dahlia-exe -b calyx --lower -l error $in -o $out");
}

rule([dahlia_setup], dahlia, c::calyx_state, "dahlia-to-calyx");
