use std::fs;
use std::io::Result;
use std::path;

fn write_primitive() -> Result<path::PathBuf> {
    // Get the OUT_DIR environment variable from Cargo.
    let base: path::PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let mut prims = base.clone();
    prims.push("primitives");
    fs::create_dir_all(&prims)?;
    // Write the compile primitives
    for (loc, src) in calyx_stdlib::KNOWN_LIBS
        .into_iter()
        .flat_map(|(_, info)| info)
        .chain(Some(calyx_stdlib::COMPILE_LIB))
    {
        let mut path = prims.clone();
        path.push(loc);
        fs::write(path, src)?;
    }
    Ok(base)
}

// The build script does not fail
fn main() {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=src/cmdline.rs");
    match write_primitive() {
        Ok(p) => {
            // The build succeeded. We're going to define the CALYX_PRIMITVE_DIR environment variable
            // so that it can be used by the compiler.
            println!(
                "cargo:rustc-env=CALYX_PRIMITIVES_DIR={}",
                p.to_string_lossy()
            );
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to create the `primitives` folder. Importing `primitives` will require passing an explicit `-l` when running the Calyx compiler. Error: {e}",
            );
        }
    }
}
