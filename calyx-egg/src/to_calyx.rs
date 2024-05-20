//! Converts from an egglog AST to Calyx.

use crate::utils;
use calyx_backend::CalyxEggBackend;
use calyx_ir;
use egglog::{match_term_app, Term, TermDag};

pub struct ToCalyx {}

pub fn program_from_egglog(program: Term, termdag: &TermDag) -> () {
    let mut converter = ToCalyx {};
    // converter.program_from_egglog(program)
}

impl ToCalyx {
    // TODO(cgyurgyik): Incomplete.
    fn parse_expr(&mut self, expr: Term, termdag: &TermDag) -> () {
        let result = match_term_app!(expr.clone();
        {
            ("Cell", [name]) => {
                println!("{:?}", expr)
            }
            ("Seq", [attributes, list]) => {
                let attributes = termdag.get(*attributes);
                println!("{:?}", expr)
            }
            ("Cons", [x, xs]) => {
                println!("{:?}", expr)
            }

            (&_, _) => todo!("unexpected: {:?}", expr)
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::utils;
    use calyx_backend::Backend;
    use main_error::MainError;
    type Result = std::result::Result<(), MainError>;
    use crate::optimize;

    #[ignore = "TODO(cgyurgyik): incomplete"]
    #[test]
    fn test_simple() -> Result {
        // TODO(cgyurgyik): Incomplete.
        // let program = to_egg::calyx_to_egglog_str(..);
        let program = "";
        let mut egraph = egglog::EGraph::default();
        egraph.parse_and_run_program(&program)?;

        let identifier: &str = "egg-main";

        let mut termdag = TermDag::default();
        let (sort, value) = egraph
            .eval_expr(&egglog::ast::Expr::Var((), identifier.into()))
            .unwrap();
        let (_, extracted) = egraph.extract(value, &mut termdag, &sort);
        println!("{}", termdag.to_string(&extracted));
        let mut converter = ToCalyx {};
        converter.parse_expr(extracted, &termdag);
        Ok(())
        // converter.parse_expr(expr)
    }
}
