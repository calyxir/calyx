import "calyx" as c;

export const mrxl_state = state("mrxl", ["mrxl"]);

fn mrxl_setup(e) {
   e.config_var_or("mrxl_flags", "mrxl.flags", "");
   e.var_("mrxl-exe", "mrxl");
   e.rule("mrxl-to-calyx", "$mrxl-exe $in ${mrxl_flags} > $out");
}

rule([mrxl_setup], mrxl_state, c::calyx_state, "mrxl-to-calyx");
