use fud_core::{cli, DriverBuilder};

fn main() -> anyhow::Result<()> {
    let mut bld = DriverBuilder::new("fud2");

    // In debug mode, get resources from the source directory.
    #[cfg(debug_assertions)]
    {
        bld.rsrc_dir(manifest_dir_macros::directory_path!("rsrc"));
    }

    // In release mode, embed resources into the binary.
    #[cfg(not(debug_assertions))]
    {
        bld.rsrc_files({
            const DIR: include_dir::Dir =
                include_dir::include_dir!("$CARGO_MANIFEST_DIR/rsrc");
            DIR.files()
                .map(|file| (file.path().to_str().unwrap(), file.contents()))
                .collect()
        });
    }

    // Get config values from cli.
    let config = cli::config_from_cli(&bld.name)?;

    bld = bld.load_plugins(&config)?;

    let driver = bld.build();
    cli::cli(&driver, &config)
}
