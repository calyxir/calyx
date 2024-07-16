import "calyx" as c;

export const yxi = state("yxi", ["yxi"]);

fn yxi_pass(in_str, out_str) {
    let s = c::calyx_setup();
    let yxi_bin = config_or("yxi", `${s.calyx_base}/target/debug/yxi`);
    shell(`${yxi_bin} -l ${s.calyx_base} ${in_str} > ${out_str}`);
}

defop calyx_to_yxi(i: c::calyx_state) >> o: yxi {
    yxi_pass("#i", "#o");
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

defop axi_wrapped(i: c::calyx_state) >> o: c::calyx_state {
    let file_name = "input";
    let tmp_yxi = replace_ext(file_name, "yxi");

    yxi_pass("#i", tmp_yxi);

    let refified_calyx = replace_ext(`refified_${file_name}`, "futil");
    c::calyx_pass("external-to-ref", "", "#i", refified_calyx);

    // For now point to standalone axi-generator.py. Can maybe turn this into a rsrc file?
    let s = c::calyx_setup();
    let dynamic = config_or("dynamic", "false");
    let generator_path = if dynamic == "true" {
        `{s.calyx_base}/yxi/axi-calyx/dynamic-axi-generator.py`
    } else {
        `${s.calyx_base}/yxi/axi-calyx/axi-generator.py`
    };

    let axi_wrapper = "axi_wrapper.futil";
    let axi_generator = config_or("axi.generator", generator_path);
    let python = config_or("python", "python3");

    shell(`${python} ${axi_generator} ${tmp_yxi} > ${axi_wrapper}`);

    let no_imports_calyx = `no_imports_${refified_calyx}`;
    shell(`sed '1,/component main/{/component main/!d; }' ${refified_calyx} > ${no_imports_calyx}`);

    shell(`cat ${axi_wrapper} ${no_imports_calyx} > #o`);
}
