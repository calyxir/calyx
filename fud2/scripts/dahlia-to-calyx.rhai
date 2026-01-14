import "calyx" as c;

export const dahlia = state("dahlia", ["fuse"]);
export let dahlia_setup = dahlia_setup;

fn dahlia_setup(e) {
   e.var_("parent-map", "parent-map.json");

   e.config_var("dahlia-exe", "dahlia");
   e.config_var_or("dahlia-args", "dahlia.args", "");
   e.rule("dahlia-to-calyx", "$dahlia-exe -b calyx --lower -l error $dahlia-args $in > $out");
}

rule([dahlia_setup], dahlia, c::calyx_state, "dahlia-to-calyx");
