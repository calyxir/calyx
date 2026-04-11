import "calyx" as c;
import "axi" as a;

fn wrapper_setup(e) {
    // Define rule for the generator.
    
    e.config_var_or("python", "python", "python3");

    e.rule("gen-mem-wrap", "$python $calyx-base/tools/mem_flat/flatten.py $in > $out",);

    // really sloppy awk a la https://unix.stackexchange.com/a/303648
    e.rule(
        "add-import",
        `awk '/import.*/,/import.*/{if (!f){print "import \"$mem-flat-path\";";f=1;print;next}}; 1 ' $in > $out`,
    );

    // use both of the required compiler passes for memory flattening
   e.rule(
       "calyx-mem-flat",
       "$calyx-exe -l $calyx-lib-path -l $mem-flat-path -p mem-flat -p dead-cell-removal $args $in > $out",
   );
	
}

fn mem_wrapped_op(e, input, output) {
   let file_name = input.split("/")[-1];
   let tmp_yxi = replace_ext(file_name, "yxi");

   e.build_cmd([tmp_yxi], "yxi", [input], []);
   let wrapper = "memwrap.futil";
   e.build_cmd([wrapper], "gen-mem-wrap", [tmp_yxi], []);

    // add memwrap as an import
   let comb_file = `${file_name}_wrap_pre.futil`;
   e.build_cmd([comb_file], "add-import", [input], [wrapper]);
   e.arg("mem-flat-path", wrapper);



   e.build_cmd([output], "calyx-mem-flat", [comb_file], [wrapper]);
   e.arg("mem-flat-path", wrapper);

    // add memwrap location as a file search path
   //e.arg("args", "mem-flat -p dead-cell-removal");
   //e.arg("pass", "dead-cell-removal");
}

op(
  "mem_flat",
  [c::calyx_setup, a::yxi_setup, wrapper_setup],
  c::calyx_state,
  c::calyx_state,
  mem_wrapped_op
);

