import "calyx" as c;

export const systolic_state = state("systolic", ["systolic"]);

fn systolic_setup(e) {
   e.config_var_or("systolic-script", "systolic.script", 
   "$calyx-base/frontends/systolic-lang/gen-systolic.py");
   e.config_var_or("python", "python", "python3");
   e.rule("systolic-to-calyx", "$python $systolic-script $in > $out");
}

rule([c::calyx_setup, systolic_setup], systolic_state, c::calyx_state, "systolic-to-calyx");
