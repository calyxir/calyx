import "calyx" as c;
import "firrtl" as f;
import "rtl_sim" as sim;

fn arcilator_setup(e) {
  e.config_var("calyx-base", "calyx.base");
  e.config_var_or("circt-dir", "circt.dir", "$calyx-base/circt");

  e.config_var_or("firtool-exe", "firtool.exe", "$circt-dir/build/bin/firtool");
  e.config_var_or("arcilator-exe", "arcilator.exe", "$circt-dir/build/bin/arcilator");
  e.config_var_or("arcilator-header-script", "arcilator.header_script", "$circt-dir/tools/arcilator/arcilator-header-cpp.py");
  e.config_var_or("arcilator-include", "arcilator.include", "$circt-dir/tools/arcilator");

  // Dynamic driver generation
  e.config_var_or("gen-driver-script", "arcilator.gen_driver", "$calyx-base/tools/firrtl/arcilator/gen_driver.py");

  let data_name = e.config_val("sim.data");
  let data_path = e.external_path(data_name);
  e.var_("sim_data", data_path);

  e.config_var_or("cider-converter", "cider.converter", "$calyx-base/target/debug/cider-data-converter");

  e.rule("arcilator-firtool", "$firtool-exe $in -ir-hw -o $out");
  e.rule("arcilator-compile", "$arcilator-exe $in -state-file=$state_file -o $ll_file");
  e.rule("arcilator-header", "python3 $arcilator-header-script $in | sed 's/)extern/)\\n\\nextern/g' > $out");
  e.rule("gen-driver", "python3 $gen-driver-script --yxi $in --out $out");
  e.rule("clang-c", "clang -Wno-override-module -c $in -o $out");
  e.rule("linking", "clang++ -I $arcilator-include -I $out_dir -I $calyx-base/tools/firrtl/arcilator $in -o $out");
  e.rule("extract-mem", "$cider-converter --to dat --output-path $out_dir $in");
  e.rule("simulation", "./$in $data_file > $out");
}

fn arcilator_sim_helper(e, input, output) {
  let dir = "arcilator-out";
  let design = `${dir}/design.mlir`;
  let mapping = `${dir}/mapping.json`;
  let sim_ll = `${dir}/simulator.ll`;
  let header = `${dir}/header.h`;
  let driver = `${dir}/driver.cpp`;
  let model_o = `${dir}/hardware_model.o`;
  let sim_bin = `${dir}/simulate`;

  e.build_cmd([design], "arcilator-firtool", [input], []);
  
  e.build_cmd([mapping, sim_ll], "arcilator-compile", [design], []);
  e.arg("state_file", mapping);
  e.arg("ll_file", sim_ll);
  
  e.build_cmd([header], "arcilator-header", [mapping], []);

  // Generate driver dynamically from YXI JSON (created by upstream firrtl-with-primitives stage)
  e.build_cmd([driver], "gen-driver", ["memory-info.json"], []);
  
  e.build_cmd([model_o], "clang-c", [sim_ll], []);
  
  e.build_cmd([sim_bin], "linking", [driver, model_o], [header]);
  e.arg("out_dir", dir);

  e.build_cmd(["sim_data_dir"], "extract-mem", ["$sim_data"], []);
  e.arg("out_dir", "sim_data_dir");
  
  // Pass the data directory (gen_driver.py produces drivers that load per-memory .dat files from a dir)
  e.build_cmd([output], "simulation", [sim_bin], ["sim_data_dir"]);
  e.arg("data_file", "sim_data_dir");
}

op(
  "arcilator",
  [c::calyx_setup, arcilator_setup],
  f::firrtl_with_primitives,
  sim::dat,
  |e, input, output| arcilator_sim_helper(e, input, output),
);
