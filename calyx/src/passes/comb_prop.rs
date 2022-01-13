use std::collections::HashMap;
use std::rc::Rc;

use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    RRC,
};

#[derive(Default)]
pub struct CombProp;

impl Named for CombProp {
    fn name() -> &'static str {
        todo!()
    }

    fn description() -> &'static str {
        todo!()
    }
}

impl Visitor for CombProp {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Build rewrites from unconditional continuous assignments.
        let port_rewrites: HashMap<(ir::Id, ir::Id), RRC<ir::Port>> = comp
            .continuous_assignments
            .iter()
            .filter(|assign| assign.guard.is_true())
            .map(|assign| {
                (assign.src.borrow().canonical(), Rc::clone(&assign.dst))
            })
            .collect();

        let cell_rewrites = HashMap::new();
        let rewriter = ir::Rewriter::new(&cell_rewrites, &port_rewrites);

        // Rewrite assignments
        comp.for_each_assignment(&|assign| {
            assign.for_each_port(|port| {
                if port.borrow().direction == ir::Direction::Output {
                    port_rewrites.get(&port.borrow().canonical()).cloned()
                } else {
                    None
                }
            })
        });
        rewriter.rewrite_control(
            &mut comp.control.borrow_mut(),
            &HashMap::new(),
            &HashMap::new(),
        );

        Ok(Action::Continue)
    }
}
