use tb::{
    cli::CLI,
    testbench::{TestbenchManager, TestbenchResult},
};

fn main() -> TestbenchResult {
    let args: CLI = argh::from_env();

    if args.version {
        println!(
            "{} v{}",
            std::env::current_exe()
                .expect("how did you call this without argv[0]??")
                .to_str()
                .expect("argv[0] not valid unicode"),
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    let tbm = TestbenchManager::new();
    tbm.run(args.using, args.input, &args.tests)
}
