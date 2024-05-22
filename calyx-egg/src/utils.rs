use calyx_backend::{Backend, CalyxEggBackend};
use main_error::MainError;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::{fmt, str::FromStr};
// TODO(cgyurgyik): Currently all the rules are in one location. These should probably be separated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RewriteRule {
    CalyxControl,
}

pub type Result = std::result::Result<(), MainError>;

impl fmt::Display for RewriteRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewriteRule::CalyxControl => write!(f, "calyx-control.egg"),
        }
    }
}

pub fn read_from(
    rule: RewriteRule,
) -> std::result::Result<String, std::io::Error> {
    fs::read_to_string(rule.to_string())
}

pub fn run_schedule(
    rules: &[RewriteRule],
) -> std::result::Result<String, std::convert::Infallible> {
    if !(rules.len() == 1 && rules[0] == RewriteRule::CalyxControl) {
        todo!("unimplemented-rules")
    }
    // TODO(cgyurgyik): This was chosen with little care.
    String::from_str(
        r#"
(run-schedule
    (saturate cell-set list analysis)
    (repeat 1024
        (saturate control list analysis)
        (run)
    )
)"#,
    )
}

pub fn calyx_to_egglog_string(
    input: &Path,
) -> std::result::Result<String, MainError> {
    // Push the rewrite rules.
    let mut program = read_from(RewriteRule::CalyxControl)?;
    let library_path =
        Path::new(option_env!("CALYX_PRIMITIVES_DIR").unwrap_or("../"));
    let ws = calyx_frontend::Workspace::construct(
        &Some(input.to_path_buf()),
        &library_path,
    )
    .unwrap();

    // Convert the Calyx program to egglog.
    let file = tempfile::NamedTempFile::new()?;
    let path = file.into_temp_path();
    let mut output = calyx_utils::OutputFile::File(path.to_path_buf());
    let ctx = calyx_ir::from_ast::ast_to_ir(ws).unwrap();
    CalyxEggBackend::emit(&ctx, &mut output).unwrap();
    let output = fs::read_to_string(path)?;

    // Push the Calyx program post-conversion.
    program.push_str(output.as_str());
    // Push the schedule.
    program.push_str(run_schedule(&[RewriteRule::CalyxControl])?.as_str());
    Ok(program)
}

pub fn calyx_string_to_egglog_string(
    input: &str,
) -> std::result::Result<String, MainError> {
    let mut temporary_file = tempfile::NamedTempFile::new()?;
    writeln!(temporary_file, "{}", input)?;
    calyx_to_egglog_string(temporary_file.path())
}

pub fn run_calyx_file_to_egglog(
    input: &Path,
    check: &str,
    display: bool,
) -> Result {
    let mut program = calyx_to_egglog_string(input)?;
    program.push_str(check);
    if display {
        println!("{}", program);
    }

    let mut egraph = egglog::EGraph::default();
    let result = egraph.parse_and_run_program(&program).map(|lines| {
        for line in lines {
            println!("{}", line);
        }
    });
    if display {
        let serialized = egraph.serialize_for_graphviz(true);
        let file = tempfile::NamedTempFile::new()?;
        let path = file.into_temp_path().with_extension("svg");
        serialized.to_svg_file(path.clone())?;
        std::process::Command::new("open")
            .arg(path.to_str().unwrap())
            .output()?;
    }
    if result.is_err() {
        println!("{:?}", result);
    }
    Ok(result?)
}

pub fn run_calyx_to_egglog_debug(input: &str, check: &str) -> Result {
    run_calyx_to_egglog_internal(input, check, true)
}

pub fn run_calyx_to_egglog(input: &str, check: &str) -> Result {
    run_calyx_to_egglog_internal(input, check, false)
}

pub fn run_calyx_to_egglog_internal(
    input: &str,
    check: &str,
    display: bool,
) -> Result {
    let mut temporary_file = tempfile::NamedTempFile::new()?;
    writeln!(temporary_file, "{}", input)?;
    run_calyx_file_to_egglog(temporary_file.path(), check, display)
}
