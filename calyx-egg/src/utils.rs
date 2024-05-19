use std::{fmt, fs, io::Error, str::FromStr};
// TODO(cgyurgyik): Currently all the rules are in one location. These should probably be separated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RewriteRule {
    CalyxControl,
}

impl fmt::Display for RewriteRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewriteRule::CalyxControl => write!(f, "calyx-control.egg"),
        }
    }
}

pub fn read_from(rule: RewriteRule) -> Result<String, std::io::Error> {
    fs::read_to_string(rule.to_string())
}

pub fn run_schedule(
    rules: &[RewriteRule],
) -> Result<String, std::convert::Infallible> {
    if !(rules.len() == 1 && rules[0] == RewriteRule::CalyxControl) {
        todo!("unimplemented-rules")
    }
    String::from_str(
        r#"
(run-schedule
    (saturate cell-set list analysis)
    (repeat 1024
        (saturate control)
        (run)
    )
)"#,
    )
}
