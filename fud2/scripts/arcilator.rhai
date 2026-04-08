import "calyx" as c;
import "firrtl" as f;
import "rtl_sim" as sim;

fn arcilator_setup(e) {
  e.config_var_or("circt-dir", "circt.dir", "$calyx-base/circt");

  e.config_var_or("firtool-exe", "firtool.exe", "$circt-dir/build/bin/firtool");
  e.config_var_or("arcilator-exe", "arcilator.exe", "$circt-dir/build/bin/arcilator");
  e.config_var_or("arcilator-header-script", "arcilator.header_script", "$circt-dir/tools/arcilator/arcilator-header-cpp.py");
  e.config_var_or("arcilator-include", "arcilator.include", "$circt-dir/tools/arcilator");

  e.config_var_or("driver", "program.driver", "$calyx-base/tools/firrtl/arcilator/driver.cpp");

  e.config_var_or("sim_data", "sim.data", "sim.data");

  e.config_var_or("cider-converter", "cider.converter", "$calyx-base/target/debug/cider-data-converter");

  e.rule("arcilator-firtool", "$firtool-exe $in -ir-hw -o $out");
  e.rule("arcilator-compile", "$arcilator-exe $in -state-file=$state_file -o $ll_file");
  e.rule("arcilator-header", "python3 $arcilator-header-script $in > $out");
  e.rule("sed", "sed 's/)extern/)\\n\\nextern/g' $in > $out");
  e.rule("copy", "cp $in $out");
  e.rule("clang-c", "clang -Wno-override-module -c $in -o $out");
  e.rule("linking", "clang++ -I $arcilator-include $driver $in -o $out");
  e.rule("extract-mem", "$cider-converter --to dat --output-path $out_dir $in");
  e.rule("simulation", "./$in $data_file > $out");
}

fn arcilator_sim_helper(e, input, output) {
  e.build_cmd(["design.mlir"], "arcilator-firtool", [input], []);
  
  e.build_cmd(["mapping.json", "simulator.ll"], "arcilator-compile", ["design.mlir"], []);
  e.arg("state_file", "mapping.json");
  e.arg("ll_file", "simulator.ll");
  
  e.build_cmd(["header.h"], "arcilator-header", ["mapping.json"], []);

  e.build_cmd(["header_fixed.h"], "sed", ["header.h"], []);

  e.build_cmd(["driver.cpp"], "copy", ["$driver"], []);
  
  e.build_cmd(["hardware_model.o"], "clang-c", ["simulator.ll"], []);
  
  e.build_cmd(["simulate"], "linking", ["hardware_model.o"], ["header_fixed.h"]);
  
  e.build_cmd(["sim_flat.data"], "extract-mem", ["$calyx-base/$sim_data"], []);

  e.build_cmd(["sim_data_dir"], "extract-mem", ["$calyx-base/$sim_data"], []);
  e.arg("out_dir", "sim_data_dir");
  
  e.build_cmd([output], "simulation", ["simulate"], ["sim_data_dir"]);
  e.arg("data_file", "sim_data_dir" + "/mem.dat");
}

op(
  "arcilator",
  [arcilator_setup],
  f::firrtl_with_primitives,
  sim::dat,
  |e, input, output| arcilator_sim_helper(e, input, output),
);
