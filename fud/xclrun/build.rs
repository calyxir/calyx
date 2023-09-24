use std::env;
use std::path::PathBuf;

fn main() {
    // TODO Don't hard-code this!
    let xrt_path = "/opt/xilinx/xrt";

    println!("cargo:rustc-link-search={}/lib", xrt_path);
    println!("cargo:rustc-link-lib=xrt_coreutil");
    println!("cargo:rerun-if-changed=wrapper.h");

    // The include path obviously needs to be made not hard-coded.
    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}/include", xrt_path)) 
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
