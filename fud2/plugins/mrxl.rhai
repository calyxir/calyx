import "calyx" as c;

export const mrxl_state = state("mrxl", ["mrxl"]);

fn mrxl_setup(e) {
   e.var_("mrxl-exe", "mrxl");
   e.rule("mrxl-to-calyx", "$mrxl-exe $in > $out");
}

rule([mrxl_setup], mrxl_state, c::calyx_state, "mrxl-to-calyx");
