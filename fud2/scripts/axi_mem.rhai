
import "calyx" as c;
import "mem_flat" as m;
import "axi" as a;

/// special case to handle situations where a file has memories that must be dimension-reduced

fn axi_wrapped_flat_op(e, input, output) {
   let file_name = input.split("/")[-1];

   let unflat_yxi = replace_ext(file_name, "yxi");
   e.build_cmd([unflat_yxi], "yxi", [input], []);

   let memwrap = "memwrap.futil";
   e.build_cmd([memwrap], "gen-mem-wrap", [unflat_yxi], []);

   let flat_pre = `flat_pre_${file_name}`;
   e.build_cmd([flat_pre], "add-import", [input], [memwrap]);
   e.arg("imp-path", memwrap);

   let flat_file = `flat_${file_name}`;
   e.build_cmd([flat_file], "calyx-mem-flat", [flat_pre], [memwrap]);
   e.arg("mem-flat-path", memwrap);

  let flat_yxi = replace_ext(`flat_${file_name}`, "yxi");
  e.build_cmd([flat_yxi], "yxi", [flat_file], []);

   let refified_calyx = replace_ext(`refified_${file_name}`, "futil");
   e.build_cmd([refified_calyx], "calyx-pass", [flat_file], []);
   e.arg("pass", "external-to-ref");
   e.arg("args", "-x external-to-ref:activate=true -m file" );

   let axi_wrapper = "axi_wrapper.futil";
   e.build_cmd([axi_wrapper], "gen-axi", [flat_yxi], []);

    let axi_pre = `axi_pre_${file_name}`;
    e.build_cmd([axi_pre], "add-import", [refified_calyx], [axi_wrapper]);
    e.arg("imp-path", axi_wrapper);


   e.build_cmd([output], "calyx-add-axi-with-mem", [axi_pre], [axi_wrapper, memwrap]);
   e.arg("axi-path", axi_wrapper);
   e.arg("mem-path", memwrap);
}

op(
  "axi-wrapped-flat",
  [c::calyx_setup, a::yxi_setup, a::axi_setup, m::mwrap_setup],
  c::calyx_state,
  c::calyx_state,
  axi_wrapped_flat_op
);


