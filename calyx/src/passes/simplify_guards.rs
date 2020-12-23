use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
};
use boolean_expression::Expr;
use ir::traversal::{Action, VisResult};
use itertools::Itertools;
use std::collections::HashSet;

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

fn extract_dnf(expr: Expr<ir::Guard>, acc: &mut Vec<Expr<ir::Guard>>) {
    match expr {
        Expr::Or(l, r) => {
            extract_dnf(*l, acc);
            extract_dnf(*r, acc);
        }
        _ => acc.push(expr),
    }
}

fn extract_cnf(expr: Expr<ir::Guard>, acc: &mut Vec<Expr<ir::Guard>>) {
    match expr {
        Expr::And(l, r) => {
            extract_cnf(*l, acc);
            extract_cnf(*r, acc);
        }
        _ => acc.push(expr),
    }
}

/// Simplify the guard using a few simple tricks.
fn simplify_guard(guard: ir::Guard) -> ir::Guard {
    // Use the BBD library to get a sum-of-product or DNF form.
    let sop = Expr::from(guard).simplify_via_bdd();
    let mut disjuncts = Vec::new();
    extract_dnf(sop, &mut disjuncts);

    // If this isn't a disjunct, return
    if disjuncts.len() == 1 {
        return disjuncts.pop().unwrap().into();
    }

    // Extract the elements for each disjunct and turn them into sets.
    let sets = disjuncts
        .into_iter()
        .map(|d| {
            let mut conjuncts = Vec::new();
            extract_cnf(d, &mut conjuncts);
            conjuncts.into_iter().collect::<HashSet<_>>()
        })
        .collect::<Vec<_>>();

    // Find common elements in all disjuncts
    let mut common = sets[0].clone();
    common.retain(|e| sets.iter().all(|s| s.contains(e)));

    // For each common factor, remove it from each disjunct and generate
    // a new guard expression.
    let not_common_guard = sets
        .into_iter()
        .map(|s| {
            s.into_iter()
                .filter_map(|e| {
                    if !common.contains(&e) {
                        Some(ir::Guard::from(e))
                    } else {
                        None
                    }
                })
                .fold(ir::Guard::True, |acc, x| acc & x)
        })
        .fold1(ir::Guard::or)
        .unwrap();

    let common_guard = common
        .into_iter()
        .fold(ir::Guard::True, |acc, x| acc & x.into());

    common_guard & not_common_guard
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
                    guard: simplify_guard(assign.guard),
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
                guard: simplify_guard(assign.guard),
            })
            .collect();
        // we don't need to traverse control
        Ok(Action::Stop)
    }
}
