
let dahlia2 = state("dahlia2", ["fuse"]);
let calyx2 = state("calyx2", ["futil"]);

fn dahlia_setup(e) {
   e.config_var("dahlia-exe", "dahlia");
   let test = e.config_val("sim.data");
   e.comment(test); 
   e.rule("dahlia2-to-calyx2", "$dahlia-exe -b calyx --lower -l error $in -o $out");
}

rule([dahlia_setup], dahlia2, calyx2, "dahlia2-to-calyx2");
