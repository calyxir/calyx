import "calyx" as c;

export const yxi = state("yxi", ["yxi"]);

op(
    "calyx-to-yxi",
    [c::calyx_setup],
    c::calyx_state,
    yxi,
    |e, input, output| {
        e.build_cmd([output], "calyx", [input], []);
        e.arg("backend", "yxi");
    },
);

fn wrapper_setup(e) {
   e.config_var_or("axi-generator", "axi.generator", "$calyx-base/yxi/axi-calyx/axi-generator.py");
   e.config_var_or("python", "python", "python3");
   e.rule("gen-axi", "$python $axi-generator $in > $out");

   // Define a simple `combine` rule that just concats any number of files.
   e.rule("combine", "cat $in > $out");
   e.rule("remove-imports", "sed '1,/component main/{/component main/!d}' $in > $out");
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

   e.build_cmd([tmp_yxi], "calyx", [input], []);
   e.arg("backend", "yxi");

   let refified_calyx = replace_ext(`refified_${file_name}`, "futil");
   e.build_cmd([refified_calyx], "calyx-pass", [input], []);
   e.arg("pass", "external-to-ref");

   let axi_wrapper = "axi_wrapper.futil";
   e.build_cmd([axi_wrapper], "gen-axi", [tmp_yxi], []);

   let no_imports_calyx = `no_imports_${refified_calyx}`;
   e.build_cmd([no_imports_calyx], "remove-imports", [refified_calyx], []);

   e.build_cmd([output], "combine", [axi_wrapper, no_imports_calyx], []);
}

op(
  "axi-wrapped",
  [c::calyx_setup, wrapper_setup],
  c::calyx_state,
  c::calyx_state,
  axi_wrapped_op
);
