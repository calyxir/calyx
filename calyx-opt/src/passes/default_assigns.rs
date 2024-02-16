use crate::analysis::AssignmentAnalysis;
use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::{CalyxResult, Error};
use std::collections::HashMap;

/// Adds default assignments to all non-`@data` ports of an instance.
pub struct DefaultAssigns {
    /// Mapping from component to data ports
    data_ports: HashMap<ir::Id, Vec<ir::Id>>,
}

impl Named for DefaultAssigns {
    fn name() -> &'static str {
        "default-assigns"
    }

    fn description() -> &'static str {
        "adds default assignments to all non-`@data` ports of an instance."
    }
}

impl ConstructVisitor for DefaultAssigns {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let data_ports = ctx
            .lib
            .signatures()
            .map(|sig| {
                let ports = sig.signature.iter().filter_map(|p| {
                    if p.attributes.has(ir::BoolAttr::Data) {
                        Some(p.name())
                    } else {
                        None
                    }
                });
                (sig.name, ports.collect())
            })
            .collect();
        Ok(Self { data_ports })
    }

    fn clear_data(&mut self) {
        /* shared across components */
    }
}

impl Visitor for DefaultAssigns {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if !comp.is_structural() {
            return Err(Error::pass_assumption(
                Self::name(),
                format!("component {} is not purely structural", comp.name),
            ));
        }

        // We only need to consider write set of the continuous assignments
        let writes = comp.continuous_assignments.iter().analysis().writes();

        // Purely structural pass
        Ok(Action::Stop)
    }
}
