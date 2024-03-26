use crate::analysis::AssignmentAnalysis;
use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::{CalyxResult, Error};
use itertools::Itertools;
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
                    if p.direction == ir::Direction::Input
                        && !p.attributes.has(ir::BoolAttr::Data)
                        && !p.attributes.has(ir::BoolAttr::Clk)
                        && !p.attributes.has(ir::BoolAttr::Reset)
                    {
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
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if !comp.is_structural() {
            return Err(Error::pass_assumption(
                Self::name(),
                format!("component {} is not purely structural", comp.name),
            ));
        }

        // We only need to consider write set of the continuous assignments
        let writes = comp
            .continuous_assignments
            .iter()
            .analysis()
            .writes()
            .group_by_cell();

        let mut assigns = Vec::new();

        let mt = vec![];
        let cells = comp.cells.iter().cloned().collect_vec();
        let mut builder = ir::Builder::new(comp, sigs);

        for cr in &cells {
            let cell = cr.borrow();
            let Some(typ) = cell.type_name() else {
                continue;
            };
            let Some(required) = self.data_ports.get(&typ) else {
                continue;
            };

            // For all the assignments not in the write set, add a default assignment
            let cell_writes = writes
                .get(&cell.name())
                .unwrap_or(&mt)
                .iter()
                .map(|p| {
                    let p = p.borrow();
                    p.name
                })
                .collect_vec();

            assigns.extend(
                required.iter().filter(|p| !cell_writes.contains(p)).map(
                    |name| {
                        let port = cell.get(name);
                        let zero = builder.add_constant(0, port.borrow().width);
                        let assign: ir::Assignment<ir::Nothing> = builder
                            .build_assignment(
                                cell.get(name),
                                zero.borrow().get("out"),
                                ir::Guard::True,
                            );
                        log::info!(
                            "Adding {}",
                            ir::Printer::assignment_to_str(&assign)
                        );
                        assign
                    },
                ),
            );
        }

        comp.continuous_assignments.extend(assigns);

        // Purely structural pass
        Ok(Action::Stop)
    }
}
