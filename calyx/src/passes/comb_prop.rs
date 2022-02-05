use std::collections::HashMap;
use std::rc::Rc;

use crate::ir::{
    self,
    traversal::{Action, VisResult, Visitor},
    RRC,
};

/// A data structure to track rewrites of ports with added functionality to declare
/// two wires to be "equal" when they are connected together.
#[derive(Default)]
struct WireRewriter {
    rewrites: ir::rewriter::PortRewriteMap,
}

impl WireRewriter {
    /// Insert into rewrite map. If `v` is in current `rewrites`, then insert `k` -> `rewrites[v]`.
    /// Panics if there is already a mapping for `k`.
    pub fn insert(&mut self, from: RRC<ir::Port>, to: RRC<ir::Port>) {
        let from_idx = from.borrow().canonical();
        let to_idx = to.borrow().canonical();

        // Should not attempt to replace keys
        if let Some(old) = self.rewrites.get(&from_idx) {
            let old_c = old.borrow().canonical();
            panic!(
                "Replacing {}.{} -> {}.{} with {}.{} -> {}.{}",
                from_idx.0,
                from_idx.1,
                old_c.0,
                old_c.1,
                from_idx.0,
                from_idx.1,
                to_idx.0,
                to_idx.1
            );
        }

        self.rewrites.insert(from_idx, to);
    }

    /// Apply all the defined equalities to the current set of rewrites.
    fn make_consistent(self) -> ir::rewriter::PortRewriteMap {
        // Perform rewrites on the defined rewrites
        self.rewrites
            .iter()
            .map(|(from, to)| {
                let to_idx = to.borrow().canonical();
                let mut final_to = self.rewrites.get(&to_idx);
                while let Some(new_to) = final_to {
                    if let Some(new_new_to) =
                        self.rewrites.get(&new_to.borrow().canonical())
                    {
                        final_to = Some(new_new_to);
                    } else {
                        break;
                    }
                }
                (from.clone(), Rc::clone(final_to.unwrap_or(to)))
            })
            /* // Remove identity rewrites
            .filter(|(k, pr)| k != &pr.borrow().canonical()) */
            .collect()
    }
}

impl From<WireRewriter> for ir::rewriter::PortRewriteMap {
    fn from(v: WireRewriter) -> Self {
        v.make_consistent()
    }
}

impl std::fmt::Debug for WireRewriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ((cell, port), port_ref) in &self.rewrites {
            writeln!(
                f,
                "{}.{} -> {}",
                cell.id,
                port.id,
                ir::Printer::port_to_str(&port_ref.borrow())
            )?
        }
        Ok(())
    }
}

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

impl ir::traversal::Named for CombProp {
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
        let mut rewrites = WireRewriter::default();

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

            let dst = assign.dst.borrow();
            let src = assign.src.borrow();
            let dst_is_wire = parent_is_wire(&dst.parent);
            let src_is_wire = parent_is_wire(&src.parent);

            if dst_is_wire {
                // src port forwards value on the wire
                rewrites.insert(
                    dst.cell_parent().borrow().get("out"),
                    Rc::clone(&assign.src),
                );
            }
            if src_is_wire {
                // wire forwards writes to dst port
                rewrites.insert(
                    src.cell_parent().borrow().get("in"),
                    Rc::clone(&assign.dst),
                );
            }
        }
        eprintln!("{:?}", rewrites);

        // Rewrite assignments
        let rewrites: ir::rewriter::PortRewriteMap = rewrites.into();
        for ((cell, port), port_ref) in &rewrites {
            eprintln!(
                "{}.{} -> {}",
                cell.id,
                port.id,
                ir::Printer::port_to_str(&port_ref.borrow())
            );
        }
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                rewrites.get(&port.borrow().canonical()).cloned()
            })
        });

        let cell_rewrites = HashMap::new();
        let rewriter = ir::Rewriter::new(&cell_rewrites, &rewrites);
        rewriter.rewrite_control(
            &mut comp.control.borrow_mut(),
            &HashMap::new(),
            &HashMap::new(),
        );

        Ok(Action::Continue)
    }
}
