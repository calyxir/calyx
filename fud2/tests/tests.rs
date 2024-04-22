use fud2::build_driver;
use fud_core::{
    config::default_config, exec::Request, run::Run, Driver, DriverBuilder,
};

fn test_driver() -> Driver {
    let mut bld = DriverBuilder::new("fud2");
    build_driver(&mut bld);
    bld.build()
}

fn request(
    driver: &Driver,
    start: &str,
    end: &str,
    through: &[&str],
) -> Request {
    fud_core::exec::Request {
        start_file: None,
        start_state: driver.get_state(start).unwrap(),
        end_file: None,
        end_state: driver.get_state(end).unwrap(),
        through: through.iter().map(|s| driver.get_op(s).unwrap()).collect(),
        workdir: ".".into(),
    }
}

fn emit_ninja(driver: &Driver, req: Request) -> String {
    let plan = driver.plan(req).unwrap();
    let config = default_config()
        .merge(("exe", "fud2"))
        .merge(("calyx.base", "/test/calyx"))
        .merge(("firrtl.exe", "/test/bin/firrtl"))
        .merge(("sim.data", "/test/data.json"))
        .merge(("xilinx.vivado", "/test/xilinx/vivado"))
        .merge(("xilinx.vitis", "/test/xilinx/vitis"))
        .merge(("xilinx.xrt", "/test/xilinx/xrt"))
        .merge(("dahlia", "/test/bin/dahlia"));
    let run = Run::with_config(driver, plan, config);
    let mut buf = vec![];
    run.emit(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

/// Get a human-readable description of a request.
fn req_desc(driver: &Driver, req: &Request) -> String {
    let mut desc = format!(
        "emit {} -> {}",
        driver.states[req.start_state].name, driver.states[req.end_state].name
    );
    if !req.through.is_empty() {
        desc.push_str(" through");
        for op in &req.through {
            desc.push(' ');
            desc.push_str(&driver.ops[*op].name);
        }
    }
    desc
}

/// Get a short string uniquely identifying a request.
fn req_slug(driver: &Driver, req: &Request) -> String {
    let mut desc = driver.states[req.start_state].name.to_string();
    for op in &req.through {
        desc.push('_');
        desc.push_str(&driver.ops[*op].name);
    }
    desc.push('_');
    desc.push_str(&driver.states[req.end_state].name);
    desc
}

fn test_emit(driver: &Driver, req: Request) {
    let desc = req_desc(driver, &req);
    let slug = req_slug(driver, &req);
    let ninja = emit_ninja(driver, req);
    insta::with_settings!({
        description => desc,
        omit_expression => true,
        snapshot_suffix => slug,
    }, {
        insta::assert_snapshot!(ninja);
    });
}

#[test]
fn calyx_to_verilog() {
    let driver = test_driver();
    test_emit(&driver, request(&driver, "calyx", "verilog", &[]));
}

#[test]
fn calyx_via_firrtl() {
    let driver = test_driver();
    test_emit(&driver, request(&driver, "calyx", "verilog", &["firrtl"]));
}

#[test]
fn sim_tests() {
    let driver = test_driver();
    for dest in &["dat", "vcd"] {
        for sim in &["icarus", "verilator"] {
            test_emit(&driver, request(&driver, "calyx", dest, &[sim]));
        }
    }
}

#[test]
fn cider_tests() {
    let driver = test_driver();
    test_emit(&driver, request(&driver, "calyx", "dat", &["interp"]));
    test_emit(&driver, request(&driver, "calyx", "debug", &[]));
}

#[test]
fn xrt_tests() {
    let driver = test_driver();
    test_emit(&driver, request(&driver, "calyx", "dat", &["xrt"]));
    test_emit(&driver, request(&driver, "calyx", "vcd", &["xrt-trace"]));
}

#[test]
fn frontend_tests() {
    let driver = test_driver();
    for frontend in &["dahlia", "mrxl"] {
        test_emit(&driver, request(&driver, frontend, "calyx", &[]));
    }
}
