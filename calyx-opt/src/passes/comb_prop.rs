use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, RRC};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

/// A data structure to track rewrites of ports with added functionality to declare
/// two wires to be "equal" when they are connected together.
#[derive(Default, Clone)]
struct WireRewriter {
    rewrites: ir::rewriter::PortRewriteMap,
}

impl WireRewriter {
    /// Insert into rewrite map. If `v` is in current `rewrites`, then insert `k` -> `rewrites[v]`.
    /// Panics if there is already a mapping for `k`.
    pub fn insert(
        &mut self,
        from: RRC<ir::Port>,
        to: RRC<ir::Port>,
    ) -> Option<RRC<ir::Port>> {
        let from_idx = from.borrow().canonical();
        self.rewrites.insert(from_idx, to)
    }

    // Removes the mapping associated with the key.
    pub fn remove(&mut self, from: RRC<ir::Port>) {
        let from_idx = from.borrow().canonical();
        self.rewrites.remove(&from_idx);
    }

    /// Apply all the defined equalities to the current set of rewrites.
    fn make_consistent(self) -> Self {
        // Perform rewrites on the defined rewrites
        let rewrites = self
            .rewrites
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
            .collect();
        Self { rewrites }
    }
}

impl From<WireRewriter> for ir::rewriter::PortRewriteMap {
    fn from(v: WireRewriter) -> Self {
        v.make_consistent().rewrites
    }
}

impl std::fmt::Debug for WireRewriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (ir::Canonical(cell, port), port_ref) in &self.rewrites {
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

/// Propagate unconditional reads and writes from wires.
///
/// If the source is a wire, we have something like:
/// ```
/// c.in = wire.out;
/// ```
/// Which means all instances of `wire.in` can be replaced with `c.in` because the wire
/// is being used to unconditionally forward values.
///
/// If the destination is a wire, then we have something like:
/// ```
/// wire.in = c.out;
/// ```
/// Which means all instances of `wire.out` can be replaced with `c.out` because the
/// wire is being used to forward values from `c.out`.
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
pub struct CombProp {
    /// Disable automatic removal of some dead assignments needed for correctness and instead mark
    /// them with @dead.
    /// NOTE: if this is enabled, the pass will not remove obviously conflicting assignments.
    do_not_eliminate: bool,
}

impl ConstructVisitor for CombProp {
    fn from(ctx: &ir::Context) -> calyx_utils::CalyxResult<Self>
    where
        Self: Sized,
    {
        let opts = Self::get_opts(ctx);
        Ok(CombProp {
            do_not_eliminate: opts[0],
        })
    }

    fn clear_data(&mut self) {
        /* do nothing */
    }
}

impl Named for CombProp {
    fn name() -> &'static str {
        "comb-prop"
    }

    fn description() -> &'static str {
        "propagate unconditional continuous assignments"
    }

    fn opts() -> &'static [(&'static str, &'static str)] {
        &[(
            "no-eliminate",
            "mark dead assignments with @dead instead of removing them",
        )]
    }
}

impl CombProp {
    /// Mark assignments for removal
    fn remove_rewritten(
        &self,
        rewritten: Vec<&RRC<ir::Port>>,
        comp: &mut ir::Component,
    ) {
        // Remove writes to all the ports that show up in write position
        if self.do_not_eliminate {
            // If elimination is disabled, mark the assignments with the @dead attribute.
            for assign in &mut comp.continuous_assignments {
                if rewritten.iter().any(|v| Rc::ptr_eq(v, &assign.dst)) {
                    assign.attributes.insert(ir::Attribute::DEAD, 1)
                }
            }
        } else {
            comp.continuous_assignments.retain_mut(|assign| {
                !rewritten.iter().any(|v| Rc::ptr_eq(v, &assign.dst))
            });
        }
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
                ir::PortParent::StaticGroup(_) => false,
            }
        };

        for assign in &mut comp.continuous_assignments {
            // Skip conditional continuous assignments
            if !assign.guard.is_true() {
                continue;
            }
            // If the destination is a wire, then we have something like:
            // ```
            // wire.in = c.out;
            // ```
            // Which means all instances of `wire.out` can be replaced with `c.out` because the
            // wire is being used to forward values from `c.out`.
            let dst = assign.dst.borrow();
            if parent_is_wire(&dst.parent) {
                rewrites.insert(
                    dst.cell_parent().borrow().get("out"),
                    Rc::clone(&assign.src),
                );
            }

            // If the source is a wire, we have something like:
            // ```
            // c.in = wire.out;
            // ```
            // Which means all instances of `wire.in` can be replaced with `c.in` because the wire
            // is being used to unconditionally forward values.
            let src = assign.src.borrow();
            if parent_is_wire(&src.parent) {
                let port = src.cell_parent().borrow().get("in");
                let old_v =
                    rewrites.insert(Rc::clone(&port), Rc::clone(&assign.dst));

                // If the insertion process found an old key, we have something like:
                // ```
                // x.in = wire.out;
                // y.in = wire.out;
                // ```
                // This means that `wire` is being used to forward values to many components and a
                // simple inlining will not work.
                if old_v.is_some() {
                    rewrites.remove(port);
                }
            }
        }

        // Rewrite assignments
        let rewrites: ir::rewriter::PortRewriteMap = rewrites.into();
        let rewritten = rewrites.values().collect_vec();
        self.remove_rewritten(rewritten, comp);

        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                rewrites.get(&port.borrow().canonical()).cloned()
            })
        });
        comp.for_each_static_assignment(|assign| {
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
            &HashMap::new(),
        );

        Ok(Action::Stop)
    }
}
