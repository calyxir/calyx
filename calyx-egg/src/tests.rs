// TODO(cgyurgyik): ???
use egglog::{ast::Expr, EGraph, ExtractReport, Function, Term, Value};

#[derive(Debug, Clone, Copy)]
enum RewriteRule {
    CalyxControl,
}

impl fmt::Display for RewriteRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewriteRule::CalyxControl => write!(f, "calyx-control.egg"),
        }
    }
}

fn test_egglog(actual: &String, expected: &String, rules: &Vec<RewriteRule>) {
    let mut egraph = EGraph::default();

    let s: String = String::new();
    for rule in rules {
        s.push(fs::read_to_string(rule.to_string()));
    }
    s.push(actual);
    s.push(format!(
        r#"
        (run-schedule
            (repeat 1024
                (saturate cell-set list analysis control)
                (run)
            )
        )"#
    ));
    s.push(format!("(= {} {})", actual, expected));

    assert!(egraph.parse_and_run_program(s));
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_simple() {
        test_egglog(
            r#"(CellSet (set-of (Cell "a"))))"#,
            r#"(CellSet (set-of (Cell "a"))))"#,
            [RewriteRule::CalyxControl],
        )
    }
}
