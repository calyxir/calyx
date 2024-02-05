use copy_dir::copy_dir;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const EXPORTED_HEADERS: [&str; 1] = ["src/btor2parser/btor2parser.h"];

fn main() {
    // we are not allowed to modify files outside of OUT_DIR,
    // so we have to copy everything to OUT_DIR before we can build it
    let immutable_source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("btor2tools");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let source_dir = out_dir.join("btor2tools-src");

    if !source_dir.exists() {
        copy_dir(immutable_source_dir, &source_dir)
            .expect("Unable to copy btor2tools sources to OUT_DIR");
    }

    run(
        "Configure btor2tools",
        Command::new(source_dir.join("configure.sh"))
            .arg("--static")
            .current_dir(&source_dir),
    );

    let build_dir = source_dir.join("build");

    run(
        "Build btor2tools",
        Command::new("make").current_dir(&build_dir),
    );

    let lib_name = "btor2parser";
    let lib_dir = build_dir.join("lib");

    // specify library search path
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Tell cargo to tell rustc to link the built shared library.
    println!("cargo:rustc-link-lib=static={}", lib_name);
    println!("cargo:lib={}", lib_dir.display());

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = EXPORTED_HEADERS
        .iter()
        .fold(bindgen::Builder::default(), |builder, header| {
            // The input header we would like to generate
            // bindings for.
            builder.header(
                source_dir
                    .join(header)
                    .to_str()
                    .expect("Unable to transform header path to string"),
            )
        })
        // export only these whitelisted types/functions
        .allowlist_function("btor2parser_.*")
        .allowlist_type("Btor2.*")
        .allowlist_function("fopen")
        .allowlist_function("fclose")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn run(description: &str, command: &mut Command) {
    println!("{}", description);
    println!("running {:?}", command);

    let success = command.status().map_or(false, |s| s.success());

    if !success {
        panic!("Error: failed to execute {:?}", command);
    }
}
