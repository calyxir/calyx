use crate::lang::context::Context;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::passes::visitor::{Named, Action, VisResult, Visitor};

#[derive(Default)]
pub struct CollapseControl {}

impl Named for CollapseControl {
    fn name() -> &'static str {
        "collapse-control"
    }

    fn description() -> &'static str {
        "Collapse nested seq and par."
    }
}

impl Visitor for CollapseControl {
    /// Collapse seq { seq { A }; B } into seq { A; B }.
    fn finish_seq(
        &mut self,
        s: &ast::Seq,
        _comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
        let mut seqs: Vec<ast::Control> = vec![];
        for con in &s.stmts {
            match con {
                ast::Control::Seq { data } => {
                    seqs.append(&mut data.stmts.clone());
                }
                _ => seqs.push(con.clone()),
            }
        }
        Ok(Action::Change(ast::Control::seq(seqs)))
    }

    /// Collapse par { par { A }; B } into par { A; B }.
    fn finish_par(
        &mut self,
        s: &ast::Par,
        _comp: &mut Component,
        _c: &Context,
    ) -> VisResult {
        let mut pars: Vec<ast::Control> = vec![];
        for con in &s.stmts {
            match con {
                ast::Control::Par { data } => {
                    pars.append(&mut data.stmts.clone());
                }
                _ => pars.push(con.clone()),
            }
        }
        Ok(Action::Change(ast::Control::par(pars)))
    }
}
