use crate::analysis::{self, AssignmentAnalysis};
use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::{CalyxResult, Error};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Tuple containing (port, set of ports).
/// When the first port is read from, all of the ports in the set must be written to.
type ReadTogether = (ir::Id, HashSet<ir::Id>);

/// Pass to check for common errors such as missing assignments to `done` holes
/// of groups.
pub struct Papercut {
    /// Map from (primitive name) -> Vec<(set of ports)>
    /// When any of the ports in a set is driven, all ports in that set must
    /// be driven.
    /// For example, when driving the `in` port of a register, the `write_en`
    /// signal must also be driven.
    write_together: HashMap<ir::Id, Vec<HashSet<ir::Id>>>,

    /// Map from (primitive name) -> Vec<(port, set of ports)>
    /// When the `port` in the tuple is being read from, all the ports in the
    /// set must be driven.
    read_together: HashMap<ir::Id, Vec<ReadTogether>>,

    /// The cells that are driven through continuous assignments
    cont_cells: HashSet<ir::Id>,
}

impl Papercut {
    #[allow(unused)]
    /// String representation of the write together and read together specifications.
    /// Used for debugging. Should not be relied upon by external users.
    fn fmt_write_together_spec(&self) -> String {
        self.write_together
            .iter()
            .map(|(prim, writes)| {
                let writes = writes
                    .iter()
                    .map(|write| {
                        write
                            .iter()
                            .sorted()
                            .map(|port| format!("{port}"))
                            .join(", ")
                    })
                    .join("; ");
                format!("{}: [{}]", prim, writes)
            })
            .join("\n")
    }
}

impl ConstructVisitor for Papercut {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let write_together =
            analysis::PortInterface::write_together_specs(ctx.lib.signatures());
        let read_together =
            analysis::PortInterface::comb_path_specs(ctx.lib.signatures())?;
        Ok(Papercut {
            write_together,
            read_together,
            cont_cells: HashSet::new(),
        })
    }

    fn clear_data(&mut self) {
        // Library specifications are shared
        self.cont_cells = HashSet::new();
    }
}

impl Named for Papercut {
    fn name() -> &'static str {
        "papercut"
    }

    fn description() -> &'static str {
        "Detect various common made mistakes"
    }
}

/// Extract information about a port.
fn port_information(
    port_ref: ir::RRC<ir::Port>,
) -> Option<((ir::Id, ir::Id), ir::Id)> {
    let port = port_ref.borrow();
    if let ir::PortParent::Cell(cell_wref) = &port.parent {
        let cell_ref = cell_wref.upgrade();
        let cell = cell_ref.borrow();
        if let ir::CellType::Primitive { name, .. } = &cell.prototype {
            return Some(((cell.name(), *name), port.name));
        }
    }
    None
}

impl Visitor for Papercut {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // If the component isn't marked "nointerface", it should have an invokable
        // interface.
        if !comp.attributes.has(ir::BoolAttr::NoInterface) && !comp.is_comb {
            // If the control program is empty, check that the `done` signal has been assigned to.
            if let ir::Control::Empty(..) = *comp.control.borrow() {
                for p in comp
                    .signature
                    .borrow()
                    .find_all_with_attr(ir::NumAttr::Done)
                {
                    let done_use =
                        comp.continuous_assignments.iter().find(|assign_ref| {
                            let assign = assign_ref.dst.borrow();
                            // If at least one assignment used the `done` port, then
                            // we're good.
                            assign.name == p.borrow().name && !assign.is_hole()
                        });
                    if done_use.is_none() {
                        return Err(Error::papercut(format!("Component `{}` has an empty control program and does not assign to the done port `{}`. Without an assignment to the done port, the component cannot return control flow.", comp.name, p.borrow().name)));
                    }
                }
            }
        }

        // For each component that's being driven in a group and comb group, make sure all signals defined for
        // that component's `write_together' and `read_together' are also driven.
        // For example, for a register, both the `.in' port and the `.write_en' port need to be
        // driven.
        for group_ref in comp.get_groups().iter() {
            let group = group_ref.borrow();
            self.check_specs(&group.assignments)
                .map_err(|err| err.with_pos(&group.attributes))?;
        }
        for group_ref in comp.get_static_groups().iter() {
            let group = group_ref.borrow();
            self.check_specs(&group.assignments)
                .map_err(|err| err.with_pos(&group.attributes))?;
        }
        for cgr in comp.comb_groups.iter() {
            let cg = cgr.borrow();
            self.check_specs(&cg.assignments)
                .map_err(|err| err.with_pos(&cg.attributes))?;
        }

        // Compute all cells that are driven in by the continuous assignments0
        self.cont_cells = comp
            .continuous_assignments
            .iter()
            .analysis()
            .cell_writes()
            .map(|cr| cr.borrow().name())
            .collect();

        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.cond.is_none() {
            let port = s.port.borrow();
            if let ir::PortParent::Cell(cell_wref) = &port.parent {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                if let ir::CellType::Primitive {
                    is_comb,
                    name: prim_name,
                    ..
                } = &cell.prototype
                {
                    // If the cell is combinational and not driven by continuous assignments
                    if *is_comb && !self.cont_cells.contains(&cell.name()) {
                        let msg = format!("Port `{}.{}` is an output port on combinational primitive `{}` and will always output 0. Add a `with` statement to the `while` statement to ensure it has a valid value during execution.", cell.name(), port.name, prim_name);
                        // Use dummy Id to get correct source location for error
                        return Err(
                            Error::papercut(msg).with_pos(&s.attributes)
                        );
                    }
                }
            }
        }
        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.cond.is_none() {
            let port = s.port.borrow();
            if let ir::PortParent::Cell(cell_wref) = &port.parent {
                let cell_ref = cell_wref.upgrade();
                let cell = cell_ref.borrow();
                if let ir::CellType::Primitive {
                    is_comb,
                    name: prim_name,
                    ..
                } = &cell.prototype
                {
                    // If the cell is combinational and not driven by continuous assignments
                    if *is_comb && !self.cont_cells.contains(&cell.name()) {
                        let msg = format!("Port `{}.{}` is an output port on combinational primitive `{}` and will always output 0. Add a `with` statement to the `if` statement to ensure it has a valid value during execution.", cell.name(), port.name, prim_name);
                        // Use dummy Id to get correct source location for error
                        return Err(
                            Error::papercut(msg).with_pos(&s.attributes)
                        );
                    }
                }
            }
        }
        Ok(Action::Continue)
    }
}

impl Papercut {
    fn check_specs<T>(&mut self, assigns: &[ir::Assignment<T>]) -> VisResult {
        let all_writes = assigns
            .iter()
            .analysis()
            .writes()
            .filter_map(port_information)
            .into_grouping_map()
            .collect::<HashSet<_>>();
        let all_reads = assigns
            .iter()
            .analysis()
            .reads()
            .filter_map(port_information)
            .into_grouping_map()
            .collect::<HashSet<_>>();
        for ((inst, comp_type), reads) in all_reads {
            if let Some(spec) = self.read_together.get(&comp_type) {
                let empty = HashSet::new();
                let writes =
                    all_writes.get(&(inst, comp_type)).unwrap_or(&empty);
                for (read, required) in spec {
                    if reads.contains(read)
                        && required.difference(writes).next().is_some()
                    {
                        let missing = required
                            .difference(writes)
                            .sorted()
                            .map(|port| format!("{}.{}", inst.clone(), port))
                            .join(", ");
                        let msg =
                            format!("Required signal not driven inside the group.\
                                        \nWhen reading the port `{}.{}', the ports [{}] must be written to.\
                                        \nThe primitive type `{}' requires this invariant.",
                                    inst,
                                    read,
                                    missing,
                                    comp_type);
                        return Err(Error::papercut(msg));
                    }
                }
            }
        }
        for ((inst, comp_type), writes) in all_writes {
            if let Some(spec) = self.write_together.get(&comp_type) {
                // For each write together spec.
                for required in spec {
                    // It should either be the case that:
                    // 1. `writes` contains no writes that overlap with `required`
                    //     In which case `required - writes` == `required`.
                    // 2. `writes` contains writes that overlap with `required`
                    //     In which case `required - writes == {}`
                    let mut diff: HashSet<_> =
                        required.difference(&writes).copied().collect();
                    if diff.is_empty() || diff == *required {
                        continue;
                    }

                    let first =
                        writes.intersection(required).sorted().next().unwrap();
                    let missing = diff
                        .drain()
                        .sorted()
                        .map(|port| format!("{}.{}", inst, port))
                        .join(", ");
                    let msg =
                        format!("Required signal not driven inside the group. \
                                 When writing to the port `{}.{}', the ports [{}] must also be written to. \
                                 The primitive type `{}' specifies this using a @write_together spec.",
                                inst,
                                first,
                                missing,
                                comp_type);
                    return Err(Error::papercut(msg));
                }
            }
        }
        // This return value is not used
        Ok(Action::Continue)
    }
}
