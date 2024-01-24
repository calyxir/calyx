use crate::analysis::{FixUp, GoDone};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;
use std::collections::HashMap;
use std::num::NonZeroU64;

/// Infer "promote_static" annotation for groups and promote control to static when
/// (conservatively) possible.
///
/// Promotion follows the current policies:
/// 1. if multiple groups enables aligned inside a seq are marked with the "promote_static"
///     attribute, then promote all promotable enables to static enables, meanwhile,
///     wrap them into a static seq
///     for example:
/// ```
///     seq {
///         a1;
///         @promote_static a2; @promote_static a3; }
/// ```
///     becomes
/// ```
///     seq {
///         a1;
///         static seq {a2; a3;}}
/// ```
/// 2. if all control statements under seq are either static statements or group enables
///     with `promote_static` annotation, then promote all group enables and turn
///     seq into static seq
/// 3. Under a par control op, all group enables marked with `promote_static` will be promoted.
///     all control statements that are either static or group enables with `promote_static` annotation
///     are wrapped inside a static par.
/// ```
/// par {@promote_static a1; a2; @promote_static a3;}
/// ```
/// becomes
/// ```
/// par {
/// static par { a1; a3; }
/// a2;
/// }
/// ```
pub struct StaticInference {
    /// Takes static information.
    static_info: FixUp,
}

// Override constructor to build latency_data information from the primitives
// library.
impl ConstructVisitor for StaticInference {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(StaticInference {
            static_info: FixUp::from_ctx(ctx),
        })
    }

    // This pass shared information between components
    fn clear_data(&mut self) {}
}

impl Named for StaticInference {
    fn name() -> &'static str {
        "static-inference"
    }

    fn description() -> &'static str {
        "infer when dynamic control programs are promotable"
    }
}

impl Visitor for StaticInference {
    // Require post order traversal of components to ensure `invoke` nodes
    // get timing information for components.
    fn iteration_order() -> Order {
        Order::Post
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        _lib: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if comp.name != "main" {
            match FixUp::get_possible_latency(&comp.control.borrow()) {
                Some(val) => {
                    comp.latency = Some(NonZeroU64::new(val).unwrap());
                    let comp_sig = comp.signature.borrow();
                    let mut done_ports: Vec<_> = comp_sig
                        .find_all_with_attr(ir::NumAttr::Done)
                        .collect();
                    let mut go_ports: Vec<_> =
                        comp_sig.find_all_with_attr(ir::NumAttr::Go).collect();
                    // XXX(Caleb): Not sure why they have to be one port.
                    if done_ports.len() == 1 && go_ports.len() == 1 {
                        let go_done = GoDone::new(vec![(
                            go_ports.pop().unwrap().borrow().name,
                            done_ports.pop().unwrap().borrow().name,
                            val,
                        )]);
                        self.static_info
                            .latency_data
                            .insert(comp.name, go_done);
                    }
                    // Insert @static attribute on the go ports.
                    for go_port in go_ports {
                        go_port
                            .borrow_mut()
                            .attributes
                            .insert(ir::NumAttr::Static, val);
                    }
                    assert_ne!(
                        0, val,
                        "Component {} has an inferred latency of 0",
                        comp.name
                    );
                    self.static_info
                        .static_component_latencies
                        .insert(comp.name, val);
                }
                None => (),
            }
        }

        Ok(Action::Continue)
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Updated components is empty
        self.static_info.fixup_timing(comp, &HashMap::new());
        Ok(Action::Continue)
    }
}
