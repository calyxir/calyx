use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    RRC,
};

/// A data structure to track rewrites of ports with added functionality to declare
/// two wires to be "equal" when they are connected together.
struct WireRewriter {}

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
        let mut port_rewrites: HashMap<ir::Canonical, RRC<ir::Port>> =
            HashMap::new();

        let parent_is_wire = |parent: &ir::PortParent| -> bool {
            match parent {
                ir::PortParent::Cell(cell_wref) => {
                    let cr = cell_wref.upgrade();
                    let cell = cr.borrow();
                    cell.is_primitive(Some("std_wire"))
                }
                ir::PortParent::Group(_) => false,
            }
        };

        for assign in &comp.continuous_assignments {
            // Skip conditional continuous assignments
            if !assign.guard.is_true() {
                continue;
            }

            // src port forwards value on the wire
            let dst = assign.dst.borrow();
            if parent_is_wire(&dst.parent) {
                if let ir::PortParent::Cell(cell_wref) = &dst.parent {
                    let cr = cell_wref.upgrade();
                    let cell = cr.borrow();
                    let dst_idx = cell.get("out").borrow().canonical();

                    // If the source has been rewritten, use the rewrite
                    // value from that instead.
                    port_rewrites.insert(dst_idx, Rc::clone(&assign.src));
                };
            }

            // wire forwards writes to dst port.
            let src = assign.src.borrow();
            if parent_is_wire(&src.parent) {
                if let ir::PortParent::Cell(cell_wref) = &src.parent {
                    let cr = cell_wref.upgrade();
                    let cell = cr.borrow();
                    let dst_idx = cell.get("in").borrow().canonical();
                    port_rewrites.insert(dst_idx, Rc::clone(&assign.dst));
                };
            }
        }

        // Make the rewrites consistent.
        let updates = port_rewrites
            .iter()
            .flat_map(|(from, to)| {
                let to_idx = to.borrow().canonical();
                let mut final_to = port_rewrites.get(&to_idx);
                while let Some(new_to) = final_to {
                    if let Some(new_new_to) =
                        port_rewrites.get(&new_to.borrow().canonical())
                    {
                        final_to = Some(new_new_to);
                    } else {
                        break;
                    }
                }
                final_to.map(|to| (from.clone(), to.clone()))
            })
            .collect_vec();

        port_rewrites.extend(updates);

        // Rewrite assignments
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                port_rewrites.get(&port.borrow().canonical()).cloned()
            })
        });

        let cell_rewrites = HashMap::new();
        let rewriter = ir::Rewriter::new(&cell_rewrites, &port_rewrites);
        rewriter.rewrite_control(
            &mut comp.control.borrow_mut(),
            &HashMap::new(),
            &HashMap::new(),
        );

        Ok(Action::Continue)
    }
}
