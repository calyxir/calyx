use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
};
use boolean_expression::Expr;
use ir::traversal::{Action, VisResult};

impl From<ir::Guard> for Expr<ir::Guard> {
    fn from(guard: ir::Guard) -> Self {
        match guard {
            ir::Guard::And(l, r) => Expr::and((*l).into(), (*r).into()),
            ir::Guard::Or(l, r) => Expr::or((*l).into(), (*r).into()),
            ir::Guard::Not(e) => Expr::not((*e).into()),
            ir::Guard::True => Expr::Const(true),
            _ => Expr::Terminal(guard),
        }
    }
}

impl From<Expr<ir::Guard>> for ir::Guard {
    fn from(expr: Expr<ir::Guard>) -> Self {
        match expr {
            Expr::Terminal(g) => g,
            Expr::And(l, r) => ir::Guard::and((*l).into(), (*r).into()),
            Expr::Or(l, r) => ir::Guard::or((*l).into(), (*r).into()),
            Expr::Not(e) => !ir::Guard::from(*e),
            Expr::Const(b) => {
                if b {
                    ir::Guard::True
                } else {
                    !ir::Guard::True
                }
            }
        }
    }
}

#[derive(Default)]
/// Adds assignments from a components `clk` port to every
/// component that contains an input `clk` port. For example
pub struct SimplifyGuards;

impl Named for SimplifyGuards {
    fn name() -> &'static str {
        "simplify-guards"
    }

    fn description() -> &'static str {
        "Aggressively simplify guards using binary decision diagrams"
    }
}

impl Visitor for SimplifyGuards {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _: &lib::LibrarySignatures,
    ) -> VisResult {
        for group in &comp.groups {
            let group_assigns = group
                .borrow_mut()
                .assignments
                .drain(..)
                .map(|assign| ir::Assignment {
                    src: assign.src,
                    dst: assign.dst,
                    guard: Expr::from(assign.guard).simplify_via_bdd().into(),
                })
                .collect();

            group.borrow_mut().assignments = group_assigns;
        }

        comp.continuous_assignments = comp
            .continuous_assignments
            .drain(..)
            .map(|assign| ir::Assignment {
                src: assign.src,
                dst: assign.dst,
                guard: Expr::from(assign.guard).simplify_via_bdd().into(),
            })
            .collect();
        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
