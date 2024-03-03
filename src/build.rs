use std::{env, fs, io::Result, path};

/// Location to install the primitives library
const PRIM_DIR: &str = "CALYX_PRIMITIVES_DIR";

struct PrimState {
    base: path::PathBuf,
    old_prims: Option<path::PathBuf>,
    new_prims: path::PathBuf,
}

/// Move the old primitives directory to a different location if present and create a new one.
fn move_primitives() -> Result<PrimState> {
    let base: path::PathBuf = match env::var_os(PRIM_DIR) {
        Some(v) => path::PathBuf::from(v),
        None => {
            let mut path: path::PathBuf = env::var_os("HOME").unwrap().into();
            path.push(".calyx");
            path
        }
    };
    let mut prims = base.clone();
    prims.push("primitives");

    // If there is already a primitives directory, move it to `old_primitves`.
    let old_prims = if prims.exists() {
        let mut old_prims = base.clone();
        old_prims.push("old_primitives");
        // If old_primitives already exists, remove it first.
        if old_prims.exists() {
            fs::remove_dir_all(&old_prims)?;
        }

        match fs::rename(&prims, &old_prims) {
            Ok(_) => (),
            Err(e) => {
                println!(
                    "cargo:warning=Failed to move primitives directory: {e}"
                );
                return Err(e);
            }
        };
        Some(old_prims)
    } else {
        None
    };

    // Create the directory again
    fs::create_dir_all(&prims)?;
    Ok(PrimState {
        base,
        old_prims,
        new_prims: prims,
    })
}

fn write_primitive(prims: &path::Path) -> Result<()> {
    // Get the OUT_DIR environment variable from Cargo.
    // Write the compile primitives
    for (loc, src) in calyx_stdlib::KNOWN_LIBS
        .into_iter()
        .flat_map(|(_, info)| info)
        .chain(Some(calyx_stdlib::COMPILE_LIB))
    {
        let mut path = prims.to_owned().clone();
        path.push(loc);
        // Make sure the parent of the file exists
        fs::create_dir_all(path.parent().unwrap())?;
        match fs::write(path, src) {
            Ok(_) => (),
            Err(e) => {
                println!(
                    "cargo:warning=Failed to write primitive: {loc}. Error: {e}"
                );
                return Err(e);
            }
        }
    }
    Ok(())
}

fn create_primitives() -> Result<path::PathBuf> {
    let PrimState {
        base,
        old_prims,
        new_prims: prims,
    } = move_primitives()?;
    match write_primitive(prims.as_path()) {
        Ok(_) => {
            if let Some(old) = old_prims {
                fs::remove_dir_all(old)?;
            }
        }
        Err(e) => {
            // Move the old primitives back
            println!("cargo:warning=Failed to write primitives directory. Restoring old directory: {e}");
            if let Some(old) = old_prims {
                fs::rename(old, &prims)?;
            }
            return Err(e);
        }
    }
    Ok(base)
}

// The build script does not fail
fn main() {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=src/cmdline.rs");
    match create_primitives() {
        Ok(p) => {
            // The build succeeded. We're going to define the CALYX_PRIMITVE_DIR environment variable
            // so that it can be used by the compiler.
            println!("cargo:rustc-env={PRIM_DIR}={}", p.to_string_lossy());
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to create the `primitives` folder. Importing `primitives` will require passing an explicit `-l` when running the Calyx compiler. Error: {e}",
            );
        }
    }
}
