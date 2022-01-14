use std::collections::HashMap;
use std::rc::Rc;

use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    RRC,
};

/// Propagate unconditional writes to the input port of `std_wire`s. Equivalent
/// to copy propagation in software compilers.
///
/// For example, we can safely inline the value `c` wherever `w.out` is read.
/// ```
/// w.in = c;
/// group g {
///   r.in = w.out
/// }
/// ```
///
/// Gets rewritten to:
/// ```
/// w.in = c;
/// group g {
///   r.in = c;
/// }
/// ```
///
/// Correctly propagates writes through mutliple wires:
/// ```
/// w1.in = c;
/// w2.in = w1.out;
/// r.in = w2.out;
/// ```
/// into:
/// ```
/// w1.in = c;
/// w2.in = c;
/// r.in = c;
/// ```
#[derive(Default)]
pub struct CombProp;

impl Named for CombProp {
    fn name() -> &'static str {
        "comb-prop"
    }

    fn description() -> &'static str {
        "propagate unconditional continuous assignments"
    }
}

impl Visitor for CombProp {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut port_rewrites: HashMap<(ir::Id, ir::Id), RRC<ir::Port>> =
            HashMap::new();

        // Build rewrites from unconditional continuous assignments to input
        // ports of `std_wire`.
        comp.continuous_assignments
            .iter()
            .filter(|assign| {
                if assign.guard.is_true() {
                    let dst = assign.dst.borrow();
                    match &dst.parent {
                        ir::PortParent::Cell(cell_wref) => {
                            let cr = cell_wref.upgrade();
                            let cell = cr.borrow();
                            cell.is_primitive(Some("std_wire"))
                        }
                        ir::PortParent::Group(_) => false,
                    }
                } else {
                    false
                }
            })
            .for_each(|assign| {
                let dst = assign.dst.borrow();
                if let ir::PortParent::Cell(cell_wref) = &dst.parent {
                    let cr = cell_wref.upgrade();
                    let cell = cr.borrow();
                    let dst_idx = cell.get("out").borrow().canonical();

                    // If the source has been rewritten, use the rewrite
                    // value from that instead.
                    let v = port_rewrites
                        .get(&assign.src.borrow().canonical())
                        .cloned();
                    if let Some(pr) = v {
                        port_rewrites.insert(dst_idx, Rc::clone(&pr));
                    } else {
                        port_rewrites.insert(dst_idx, Rc::clone(&assign.src));
                    }
                };
            });

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
