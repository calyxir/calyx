use crate::analysis::{GoDone, InferenceAnalysis};
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;
use itertools::Itertools;

/// Infer @promotable annotation
/// for groups and control.
/// Inference occurs whenever possible.
pub struct StaticInference {
    /// Takes static information.
    inference_analysis: InferenceAnalysis,
}

// Override constructor to build latency_data information from the primitives
// library.
impl ConstructVisitor for StaticInference {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(StaticInference {
            inference_analysis: InferenceAnalysis::from_ctx(ctx),
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
            // If the entire component's control is promotable.
            if let Some(val) =
                InferenceAnalysis::get_possible_latency(&comp.control.borrow())
            {
                let comp_sig = comp.signature.borrow();
                let mut go_ports: Vec<_> =
                    comp_sig.find_all_with_attr(ir::NumAttr::Go).collect();
                // Insert @promotable attribute on the go ports.
                for go_port in &mut go_ports {
                    go_port
                        .borrow_mut()
                        .attributes
                        .insert(ir::NumAttr::Promotable, val);
                }
                let mut done_ports: Vec<_> =
                    comp_sig.find_all_with_attr(ir::NumAttr::Done).collect();
                // Update `latency_data`.
                go_ports.sort_by_key(|port| {
                    port.borrow().attributes.get(ir::NumAttr::Go).unwrap()
                });
                done_ports.sort_by_key(|port| {
                    port.borrow().attributes.get(ir::NumAttr::Done).unwrap()
                });
                let zipped: Vec<_> =
                    go_ports.iter().zip(done_ports.iter()).collect();
                let go_done_ports = zipped
                    .into_iter()
                    .map(|(go_port, done_port)| {
                        (go_port.borrow().name, done_port.borrow().name, val)
                    })
                    .collect_vec();
                self.inference_analysis.add_component((
                    comp.name,
                    val,
                    GoDone::new(go_done_ports),
                ));
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
        // ``Fix up the timing'', but with the updated_components argument as
        // and empty HashMap. This just performs inference.
        self.inference_analysis.fixup_timing(comp);
        Ok(Action::Continue)
    }
}
