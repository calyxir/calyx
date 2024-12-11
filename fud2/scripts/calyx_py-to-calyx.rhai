import "calyx" as c;

export const calyx_py = state("calyx_py", ["py"]);

fn calyx_py_setup(e) {
   e.config_var_or("py_args", "py.args", "");
   e.rule("calyx_py-to-calyx", "python3 $in $py_args > $out");
}

rule([calyx_py_setup], calyx_py, c::calyx_state, "calyx_py-to-calyx");
