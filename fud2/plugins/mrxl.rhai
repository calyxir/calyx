let mrxl = state("mrxl", ["mrxl"]);

fn mrxl_setup(e) {
   e.var_("mrxl-exe", "mrxl");
   e.rule("mrxl-to-calyx", "$mrxl-exe $in > $out");
}

rule([mrxl_setup], mrxl, get_state("calyx"), "mrxl-to-calyx");
