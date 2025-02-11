use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, RRC};
use itertools::Itertools;
use std::rc::Rc;

/// A data structure to track rewrites of ports with added functionality to declare
/// two wires to be "equal" when they are connected together.
#[derive(Default, Clone)]
struct WireRewriter {
    rewrites: ir::rewriter::PortRewriteMap,
}

impl WireRewriter {
    // If the destination is a wire, then we have something like:
    // ```
    // wire.in = c.out;
    // ```
    // Which means all instances of `wire.out` can be replaced with `c.out` because the
    // wire is being used to forward values from `c.out`.
    pub fn insert_src_rewrite(
        &mut self,
        wire: RRC<ir::Cell>,
        src: RRC<ir::Port>,
    ) {
        let wire_out = wire.borrow().get("out");
        log::debug!(
            "src rewrite: {} -> {}",
            wire_out.borrow().canonical(),
            src.borrow().canonical(),
        );
        let old = self.insert(wire_out, Rc::clone(&src));
        assert!(
            old.is_none(),
            "Attempting to add multiple sources to a wire"
        );
    }

    // If the source is a wire, we have something like:
    // ```
    // c.in = wire.out;
    // ```
    // Which means all instances of `wire.in` can be replaced with `c.in` because the wire
    // is being used to unconditionally forward values.
    pub fn insert_dst_rewrite(
        &mut self,
        wire: RRC<ir::Cell>,
        dst: RRC<ir::Port>,
    ) {
        let wire_in = wire.borrow().get("in");
        log::debug!(
            "dst rewrite: {} -> {}",
            wire_in.borrow().canonical(),
            dst.borrow().canonical(),
        );
        let old_v = self.insert(Rc::clone(&wire_in), dst);

        // If the insertion process found an old key, we have something like:
        // ```
        // x.in = wire.out;
        // y.in = wire.out;
        // ```
        // This means that `wire` is being used to forward values to many components and a
        // simple inlining will not work.
        if old_v.is_some() {
            self.remove(wire_in);
        }

        // No forwading generated because the wire is used in dst position
    }

    /// Insert into rewrite map. If `v` is in current `rewrites`, then insert `k` -> `rewrites[v]`
    /// and returns the previous rewrite if any.
    fn insert(
        &mut self,
        from: RRC<ir::Port>,
        to: RRC<ir::Port>,
    ) -> Option<RRC<ir::Port>> {
        let from_idx = from.borrow().canonical();
        let old = self.rewrites.insert(from_idx, to);
        if log::log_enabled!(log::Level::Debug) {
            if let Some(ref old) = old {
                log::debug!(
                    "Previous rewrite: {} -> {}",
                    from.borrow().canonical(),
                    old.borrow().canonical()
                );
            }
        }
        old
    }

    // Removes the mapping associated with the key.
    pub fn remove(&mut self, from: RRC<ir::Port>) {
        log::debug!("Removing rewrite for `{}'", from.borrow().canonical());
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
        for (ir::Canonical { cell, port }, port_ref) in &self.rewrites {
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
    no_eliminate: bool,
}

impl ConstructVisitor for CombProp {
    fn from(ctx: &ir::Context) -> calyx_utils::CalyxResult<Self>
    where
        Self: Sized,
    {
        let opts = Self::get_opts(ctx);
        Ok(CombProp {
            no_eliminate: opts[&"no-eliminate"].bool(),
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

    fn opts() -> Vec<PassOpt> {
        vec![PassOpt::new(
            "no-eliminate",
            "mark dead assignments with @dead instead of removing them",
            ParseVal::Bool(false),
            PassOpt::parse_bool,
        )]
    }
}

impl CombProp {
    /// Predicate for removing an assignment
    #[inline]
    fn remove_predicate<T>(
        rewritten: &[RRC<ir::Port>],
        assign: &ir::Assignment<T>,
    ) -> bool
    where
        T: Clone + Eq + ToString,
    {
        let out = rewritten.iter().any(|v| Rc::ptr_eq(v, &assign.dst));
        if log::log_enabled!(log::Level::Debug) && out {
            log::debug!("Removing: {}", ir::Printer::assignment_to_str(assign));
        }
        out
    }

    /// Mark assignments for removal
    fn remove_rewritten(
        &self,
        rewritten: &[RRC<ir::Port>],
        comp: &mut ir::Component,
    ) {
        log::debug!(
            "Rewritten: {}",
            rewritten
                .iter()
                .map(|p| format!("{}", p.borrow().canonical()))
                .collect::<Vec<_>>()
                .join(", ")
        );
        // Remove writes to all the ports that show up in write position
        if self.no_eliminate {
            // If elimination is disabled, mark the assignments with the @dead attribute.
            for assign in &mut comp.continuous_assignments {
                if Self::remove_predicate(rewritten, assign) {
                    assign.attributes.insert(ir::InternalAttr::DEAD, 1)
                }
            }
        } else {
            comp.continuous_assignments.retain_mut(|assign| {
                !Self::remove_predicate(rewritten, assign)
            });
        }
    }

    fn parent_is_wire(parent: &ir::PortParent) -> bool {
        match parent {
            ir::PortParent::Cell(cell_wref) => {
                let cr = cell_wref.upgrade();
                let cell = cr.borrow();
                cell.is_primitive(Some("std_wire"))
            }
            ir::PortParent::Group(_) => false,
            ir::PortParent::StaticGroup(_) => false,
        }
    }

    fn disable_rewrite<T>(
        assign: &mut ir::Assignment<T>,
        rewrites: &mut WireRewriter,
    ) {
        if assign.guard.is_true() {
            return;
        }
        assign.for_each_port(|pr| {
            let p = pr.borrow();
            if p.direction == ir::Direction::Output
                && Self::parent_is_wire(&p.parent)
            {
                let cell = p.cell_parent();
                rewrites.remove(cell.borrow().get("in"));
            }
            // Never change the port
            None
        });
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

        for assign in &mut comp.continuous_assignments {
            // Cannot add rewrites for conditional statements
            if !assign.guard.is_true() {
                continue;
            }

            let dst = assign.dst.borrow();
            if Self::parent_is_wire(&dst.parent) {
                rewrites.insert_src_rewrite(
                    dst.cell_parent(),
                    Rc::clone(&assign.src),
                );
            }

            let src = assign.src.borrow();
            if Self::parent_is_wire(&src.parent) {
                rewrites.insert_dst_rewrite(
                    src.cell_parent(),
                    Rc::clone(&assign.dst),
                );
            }
        }

        // Disable all rewrites:
        // If the statement uses a wire output (w.out) as a source, we
        // cannot rewrite the wire's input (w.in) uses
        comp.for_each_assignment(|assign| {
            Self::disable_rewrite(assign, &mut rewrites)
        });
        comp.for_each_static_assignment(|assign| {
            Self::disable_rewrite(assign, &mut rewrites)
        });

        // Rewrite assignments
        // Make the set of rewrites consistent and transform into map
        let rewrites: ir::rewriter::PortRewriteMap = rewrites.into();
        let rewritten = rewrites.values().cloned().collect_vec();
        self.remove_rewritten(&rewritten, comp);

        comp.for_each_assignment(|assign| {
            if !assign.attributes.has(ir::InternalAttr::DEAD) {
                assign.for_each_port(|port| {
                    rewrites.get(&port.borrow().canonical()).cloned()
                })
            }
        });
        comp.for_each_static_assignment(|assign| {
            if !assign.attributes.has(ir::InternalAttr::DEAD) {
                assign.for_each_port(|port| {
                    rewrites.get(&port.borrow().canonical()).cloned()
                })
            }
        });

        let rewriter = ir::Rewriter {
            port_map: rewrites,
            ..Default::default()
        };
        rewriter.rewrite_control(&mut comp.control.borrow_mut());

        Ok(Action::Stop)
    }
}
