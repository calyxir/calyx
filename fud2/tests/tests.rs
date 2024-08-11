use figment::providers::Format as _;
use fud_core::{
    config::default_config,
    exec::{
        plan::{EnumeratePlanner, FindPlan, LegacyPlanner},
        Plan, Request, IO,
    },
    run::Run,
    Driver, DriverBuilder,
};
use itertools::Itertools;

#[cfg(not(feature = "migrate_to_scripts"))]
fn test_driver() -> Driver {
    let mut bld = DriverBuilder::new("fud2");
    fud2::build_driver(&mut bld);
    bld.build()
}

#[cfg(feature = "migrate_to_scripts")]
fn test_driver() -> Driver {
    let mut bld = DriverBuilder::new("fud2-plugins");
    let config = figment::Figment::new();
    bld.scripts_dir(manifest_dir_macros::directory_path!("scripts"));
    bld.load_plugins(&config).unwrap().build()
}

fn driver_from_path_with_config(
    path: &str,
    config: figment::Figment,
) -> Driver {
    let mut bld = DriverBuilder::new("fud2-plugins");
    let path = format!(
        "{}/{}",
        manifest_dir_macros::directory_path!("tests/scripts"),
        path
    );
    bld.scripts_dir(&path);
    bld.load_plugins(&config).unwrap().build()
}

fn driver_from_path(path: &str) -> Driver {
    driver_from_path_with_config(path, figment::Figment::new())
}

trait InstaTest: Sized {
    /// Get a human-readable description of Self
    fn desc(&self, driver: &Driver) -> String;

    /// Get a short string uniquely identifying Self
    fn slug(&self, driver: &Driver) -> String;

    /// Emit the string that will be snapshot tested
    fn emit(self, driver: &Driver) -> String;

    /// Run snapshot test
    fn test(self, driver: &Driver) {
        let desc = self.desc(driver);
        let slug = self.slug(driver);
        let snapshot = self.emit(driver);
        insta::with_settings!({
            description => desc,
            omit_expression => true,
            snapshot_suffix => format!("{slug}"),
        }, {
            insta::assert_snapshot!(snapshot);
        });
    }
}

impl InstaTest for Plan {
    fn desc(&self, driver: &Driver) -> String {
        let ops = self
            .steps
            .iter()
            .map(|(opref, _, _)| driver.ops[*opref].name.to_string())
            .collect_vec()
            .join(" -> ");
        format!("emit plan: {ops}")
    }

    fn slug(&self, driver: &Driver) -> String {
        let ops = self
            .steps
            .iter()
            .map(|(opref, _, _)| driver.ops[*opref].name.to_string())
            .collect_vec()
            .join("_");
        format!("plan_{ops}")
    }

    fn emit(self, driver: &Driver) -> String {
        let config = default_config()
            .merge(("exe", "fud2"))
            .merge(("calyx.base", "/test/calyx"))
            .merge(("firrtl.exe", "/test/bin/firrtl"))
            .merge(("sim.data", "/test/data.json"))
            .merge(("xilinx.vivado", "/test/xilinx/vivado"))
            .merge(("xilinx.vitis", "/test/xilinx/vitis"))
            .merge(("xilinx.xrt", "/test/xilinx/xrt"))
            .merge(("dahlia", "/test/bin/dahlia"))
            .merge(("c0", "v1"));
        let run = Run::with_config(driver, self, config);
        let mut buf = vec![];
        run.emit(&mut buf).unwrap();
        // turn into string, and remove comments
        String::from_utf8(buf)
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl InstaTest for Request {
    fn desc(&self, driver: &Driver) -> String {
        let start_str = self
            .start_states
            .iter()
            .map(|&state| &driver.states[state].name)
            .join(" ");
        let end_str = &self
            .end_states
            .iter()
            .map(|&state| &driver.states[state].name)
            .join(" ");
        let mut desc = format!("emit request: {} -> {}", start_str, end_str);
        if !self.through.is_empty() {
            desc.push_str(" through");
            for op in &self.through {
                desc.push(' ');
                desc.push_str(&driver.ops[*op].name);
            }
        }
        desc
    }

    fn slug(&self, driver: &Driver) -> String {
        let mut desc = self
            .start_states
            .iter()
            .map(|&state| &driver.states[state].name)
            .join("_");
        if !self.through.is_empty() {
            desc.push_str("_through");
            for op in &self.through {
                desc.push('_');
                desc.push_str(&driver.ops[*op].name);
            }
        }
        desc.push_str("_to_");
        desc.push_str(
            &self
                .end_states
                .iter()
                .map(|&state| &driver.states[state].name)
                .join("_"),
        );
        desc
    }

    fn emit(self, driver: &Driver) -> String {
        let plan = driver.plan(self).unwrap();
        plan.emit(driver)
    }
}

fn request_with_planner(
    driver: &Driver,
    start: &[&str],
    end: &[&str],
    through: &[&str],
    planner: impl FindPlan + 'static,
) -> Request {
    fud_core::exec::Request {
        start_files: vec![],
        start_states: start
            .iter()
            .map(|s| driver.get_state(s).unwrap())
            .collect(),
        end_files: vec![],
        end_states: end.iter().map(|s| driver.get_state(s).unwrap()).collect(),
        through: through.iter().map(|s| driver.get_op(s).unwrap()).collect(),
        workdir: ".".into(),
        planner: Box::new(planner),
    }
}

fn request(
    driver: &Driver,
    start: &[&str],
    end: &[&str],
    through: &[&str],
) -> Request {
    request_with_planner(driver, start, end, through, LegacyPlanner {})
}

#[test]
fn all_ops() {
    let driver = test_driver();
    for op in driver.ops.keys() {
        let plan = Plan {
            steps: vec![(
                op,
                vec![IO::File("/input.ext".into())],
                vec![IO::File("/output.ext".into())],
            )],
            workdir: ".".into(),
            inputs: vec![IO::File("/input.ext".into())],
            results: vec![IO::File("/output.ext".into())],
        };
        plan.test(&driver);
    }
}

#[test]
fn list_states() {
    let driver = test_driver();
    let states = driver
        .states
        .values()
        .map(|state| &state.name)
        .sorted()
        .collect::<Vec<_>>();
    insta::with_settings!({
        omit_expression => true
    }, {
        insta::assert_debug_snapshot!(states)
    });
}

#[test]
fn list_ops() {
    let driver = test_driver();
    let ops = driver
        .ops
        .values()
        .map(|op| {
            (
                &op.name,
                &driver.states[op.input[0]].name,
                &driver.states[op.output[0]].name,
            )
        })
        .sorted()
        .collect::<Vec<_>>();
    insta::with_settings!({
        omit_expression => true
    }, {
        insta::assert_debug_snapshot!(ops)
    });
}

#[test]
fn calyx_to_verilog() {
    let driver = test_driver();
    request(&driver, &["calyx"], &["verilog"], &[]).test(&driver);
}

#[test]
fn calyx_via_firrtl() {
    let driver = test_driver();
    request(&driver, &["calyx"], &["verilog-refmem"], &["firrtl"])
        .test(&driver);
}

#[test]
fn sim_tests() {
    let driver = test_driver();
    for dest in &["dat", "vcd"] {
        for sim in &["icarus", "verilator"] {
            request(&driver, &["calyx"], &[dest], &[sim]).test(&driver);
        }
    }
}

#[test]
fn cider_tests() {
    let driver = test_driver();
    request(&driver, &["calyx"], &["dat"], &["cider"]).test(&driver);
    request(&driver, &["calyx"], &["cider-debug"], &[]).test(&driver);
}

#[test]
fn xrt_tests() {
    let driver = test_driver();
    request(&driver, &["calyx"], &["dat"], &["xrt"]).test(&driver);
    request(&driver, &["calyx"], &["vcd"], &["xrt-trace"]).test(&driver);
}

#[test]
fn frontend_tests() {
    let driver = test_driver();
    for frontend in &["dahlia", "mrxl"] {
        request(&driver, &[frontend], &["calyx"], &[]).test(&driver);
    }
}

#[test]
fn shell_deps_tests() {
    let driver = driver_from_path("shell_deps");
    request(&driver, &["s1"], &["s2"], &[]).test(&driver);
    request(&driver, &["s3"], &["s4"], &[]).test(&driver);
}

#[test]
fn shell_tests() {
    let driver = driver_from_path("shell_deps");
    request(&driver, &["s5"], &["s6"], &[]).test(&driver);
}

#[test]
fn simple_defops() {
    let driver = driver_from_path("defop");
    request(&driver, &["state0"], &["state1"], &[]).test(&driver);
    request_with_planner(
        &driver,
        &["state0", "state1"],
        &["state2"],
        &[],
        EnumeratePlanner {},
    )
    .test(&driver);
    request_with_planner(
        &driver,
        &["state0"],
        &["state2", "state1"],
        &[],
        EnumeratePlanner {},
    )
    .test(&driver);
    request_with_planner(
        &driver,
        &["state0", "state1", "state2"],
        &["state3", "state4"],
        &[],
        EnumeratePlanner {},
    )
    .test(&driver);
}

#[test]
fn config() {
    let config = figment::Figment::from({
        let source = r#"
                c0 = "v0"
            "#;
        figment::providers::Toml::string(source)
    });
    let driver = driver_from_path_with_config("defop", config);
    request(&driver, &["state0"], &["state1"], &["t4"]).test(&driver);
    request(&driver, &["state0"], &["state1"], &["t5"]).test(&driver);
}
