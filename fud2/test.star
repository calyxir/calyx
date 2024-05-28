# define the dahlia state
dahlia = state(
    name = "dahlia2",
    extensions = ["fuse"]
)

def dahlia_setup(e):
    e.config_var("dahlia-exe", "dahlia")
    e.rule("dahlia-to-calyx", "$dahlia-exe -b calyx --lower -l error $in -o $out")
    return e

rule([dahlia_setup], dahlia, get_state("calyx"), "dahlia2-to-calyx")
