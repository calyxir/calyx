import "calyx" as c;

export const yxi = state("yxi", ["yxi"]);

export let yxi_setup = yxi_setup;
fn yxi_setup(e) {
   e.config_var_or("yxi", "yxi", "$calyx-base/target/debug/yxi");
   e.rule("yxi", "$yxi -l $calyx-base $in > $out");
}

op(
    "calyx-to-yxi",
    [c::calyx_setup, yxi_setup],
    c::calyx_state,
    yxi,
    |e, input, output| {
        e.build_cmd([output], "yxi", [input], []);
    },
);

export let axi_setup = wrapper_setup;
fn wrapper_setup(e) {
    // Define a `gen-axi` rule that invokes our Python code generator program.
    // For now point to standalone axi-generator.py. Can maybe turn this into a rsrc file?
    let dynamic =
        e.config_constrained_or("dynamic", ["true", "false"], "false");
    
    let generator_path = if dynamic == "true" {
        "$calyx-base/yxi/axi-calyx/dynamic_axi_generator.py"
    } else {
        "$calyx-base/yxi/axi-calyx/axi_generator.py"
    };
    
    let for_xilinx = e.config_constrained_or("xilinx.controlled", ["true", "false"], "false");
    
    if for_xilinx == "true" {
        e.var_("python_args", "--generate-for-xilinx");
    } else {
        e.var_("python_args", "");
    };
    
    e.config_var_or("axi-generator", "axi.generator", generator_path);
    e.config_var_or("python", "python", "python3");

    e.rule("gen-axi", "$python $axi-generator $python_args $in > $out");
    e.rule(
            "add-import",
            `awk '/import.*/,/import.*/{if (!f){print "import \"$imp-path\";";f=1;print;next}}; 1 ' $in > $out`,
        );

    e.rule(
           "calyx-add-axi",
           "$calyx-exe -l $calyx-lib-path -l $axi-path -p none $args $in > $out",
    );

    e.rule(
           "calyx-add-axi-with-mem",
           "$calyx-exe -l $calyx-lib-path -l $axi-path -l $mem-path -p none $args $in > $out",
    );


}

/// Replace the extension in `path` with `new_ext`
fn replace_ext(path, new_ext) {
   if "." in path {
      let no_ext = path.split_rev(".", 2)[-1];
      return `${no_ext}.${new_ext}`;
   } else {
      return `${path}.${new_ext}`;
   }
}

fn axi_wrapped_op(e, input, output) {
   let file_name = input.split("/")[-1];
   let tmp_yxi = replace_ext(file_name, "yxi");

   e.build_cmd([tmp_yxi], "yxi", [input], []);

   let refified_calyx = replace_ext(`refified_${file_name}`, "futil");
   e.build_cmd([refified_calyx], "calyx-pass", [input], []);
   e.arg("pass", "external-to-ref");
   e.arg("args", "-x external-to-ref:activate=true -m file" );

   let axi_wrapper = "axi_wrapper.futil";
   e.build_cmd([axi_wrapper], "gen-axi", [tmp_yxi], []);

   let with_import_calyx = `with_axi_${refified_calyx}`;
   e.build_cmd([with_import_calyx], "add-import", [refified_calyx], [axi_wrapper]);
   e.arg("imp-path", axi_wrapper);


   e.build_cmd([output], "calyx-add-axi", [with_import_calyx], []);
   e.arg("axi-path", "axi_wrapper.futil");
}

op(
  "axi-wrapped",
  [c::calyx_setup, yxi_setup, wrapper_setup],
  c::calyx_state,
  c::calyx_state,
  axi_wrapped_op
);
