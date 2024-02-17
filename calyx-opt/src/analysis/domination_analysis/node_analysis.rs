use crate::analysis::{
    AssignmentAnalysis, DominatorMap, ReadWriteSet, ShareSet,
};
use calyx_ir as ir;
use std::collections::HashSet;

const BEGIN_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::BEGIN_ID);
const END_ID: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::END_ID);

// This file contains analysis that reasons about reads and writes to given "nodes"
// of control statements. In other words, it reasons about control statements,
// assuming that NODE_ID,BEGIN_ID, and END_ID attributes have been attached to
// each of the control statements. This type of node labelling is mainly used in
// DominatorMap.

// Inputs are a control statement c and a u64 id. If control is an if statment, then
// the id should refer to either the begin or end id of stmt c. Returns true if id refers
// to the begin id and false if it refers to the end id. If it is not an if statement
// return true.
fn not_end_id(c: &ir::Control, id: u64) -> bool {
    match c {
        ir::Control::If(if_control) => {
            if let Some(begin) = if_control.attributes.get(BEGIN_ID) {
                if begin == id {
                    return true;
                }
            }
            if let Some(end) = if_control.attributes.get(END_ID) {
                if end == id {
                    return false;
                }
            }
            unreachable!("id should match either beginning or ending id")
        }
        _ => true,
    }
}

// Inputs are a control statement c and a u64 id. If control is an if statment, then
// the id should refer to either the begin or end id of stmt c. Returns true if id refers
// to the begin id and false if it refers to the end id. If it is not an if statement
// return true.
fn not_end_id_static(c: &ir::StaticControl, id: u64) -> bool {
    match c {
        ir::StaticControl::If(if_control) => {
            if let Some(begin) = if_control.attributes.get(BEGIN_ID) {
                if begin == id {
                    return true;
                }
            }
            if let Some(end) = if_control.attributes.get(END_ID) {
                if end == id {
                    return false;
                }
            }
            unreachable!("id should match either beginning or ending id")
        }
        _ => true,
    }
}

//if the assignment reads only dones, return true. This is used so that we
//can ignore reads of "done" cells.
fn reads_only_dones<T>(assignment: &ir::Assignment<T>) -> bool {
    ReadWriteSet::port_reads(assignment).all(|port| done_or_const(&port))
}

// Returns true if port is a "done" port or is a constant
fn done_or_const(port: &ir::RRC<ir::Port>) -> bool {
    port.borrow().attributes.has(ir::NumAttr::Done)
        || port.borrow().is_constant(1, 1)
}

//Adds the ids of any state_shareable cells that are read from in assignments,
//excluding reads where the only reads are from "done" ports.
fn add_assignment_reads<T>(
    reads: &mut HashSet<ir::Id>,
    share: &ShareSet,
    assignments: &[ir::Assignment<T>],
) {
    let assigns = assignments
        .iter()
        .filter(|assign| !reads_only_dones(assign));

    for cell in assigns.analysis().cell_reads() {
        if share.is_shareable_component(&cell) && !cell.borrow().is_reference()
        {
            reads.insert(cell.borrow().name());
        }
    }
}

//given a port, insert the port's parent's id if the parent the port's parent
//is shareable
fn add_parent_if_shareable(
    reads: &mut HashSet<ir::Id>,
    share: &ShareSet,
    port: &ir::RRC<ir::Port>,
) {
    if let ir::PortParent::Cell(cell) = &port.borrow().parent {
        if share.is_shareable_component(&cell.upgrade()) {
            reads.insert(cell.upgrade().borrow().name());
        }
    }
}

///Contains the ids of all the cells that are read from in a given "node".
pub struct NodeReads;

impl NodeReads {
    // Given a node n, gets the reads of shareable components that occur in n,
    // excluding reads of the done port
    pub fn get_reads_of_node(
        node: &u64,
        comp: &mut ir::Component,
        state_shareable: &ShareSet,
    ) -> HashSet<ir::Id> {
        let mut reads: HashSet<ir::Id> = HashSet::new();
        match DominatorMap::get_control(*node, &comp.control.borrow()) {
            None => (),
            Some(ir::GenericControl::Dynamic(c)) => match c {
                ir::Control::Empty(_)
                | ir::Control::Seq(_)
                | ir::Control::Par(_)
                | ir::Control::Repeat(_)
                | ir::Control::Static(_) => {
                    unreachable!(
                        "no empty/seqs/pars/static should be in domination map"
                    )
                }
                ir::Control::If(ir::If { port, cond, .. })
                | ir::Control::While(ir::While { port, cond, .. }) => {
                    if not_end_id(c, *node) {
                        add_parent_if_shareable(
                            &mut reads,
                            state_shareable,
                            port,
                        );
                        if let Some(group) = cond {
                            add_assignment_reads(
                                &mut reads,
                                state_shareable,
                                &group.borrow().assignments,
                            );
                        }
                    }
                }
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    add_assignment_reads(
                        &mut reads,
                        state_shareable,
                        &group.borrow().assignments,
                    );
                }
                ir::Control::Invoke(ir::Invoke {
                    comp,
                    inputs,
                    outputs,
                    comb_group,
                    ..
                }) => {
                    for (_, port) in inputs.iter() {
                        add_parent_if_shareable(
                            &mut reads,
                            state_shareable,
                            port,
                        );
                    }
                    if !outputs.is_empty()
                        && state_shareable.is_shareable_component(comp)
                    {
                        reads.insert(comp.borrow().name());
                    }
                    if let Some(group) = comb_group {
                        add_assignment_reads(
                            &mut reads,
                            state_shareable,
                            &group.borrow().assignments,
                        );
                    }
                }
            },
            Some(ir::GenericControl::Static(sc)) => match sc {
                ir::StaticControl::Invoke(ir::StaticInvoke {
                    comp,
                    inputs,
                    outputs,
                    ..
                }) => {
                    for (_, port) in inputs.iter() {
                        add_parent_if_shareable(
                            &mut reads,
                            state_shareable,
                            port,
                        );
                    }
                    if !outputs.is_empty()
                        && state_shareable.is_shareable_component(comp)
                    {
                        reads.insert(comp.borrow().name());
                    }
                }
                ir::StaticControl::Enable(ir::StaticEnable {
                    group, ..
                }) => {
                    add_assignment_reads(
                        &mut reads,
                        state_shareable,
                        &group.borrow().assignments,
                    );
                }
                ir::StaticControl::If(ir::StaticIf { port, .. }) => {
                    if not_end_id_static(sc, *node) {
                        add_parent_if_shareable(
                            &mut reads,
                            state_shareable,
                            port,
                        );
                    }
                }
                ir::StaticControl::Empty(_)
                | ir::StaticControl::Par(_)
                | ir::StaticControl::Seq(_)
                | ir::StaticControl::Repeat(_) => unreachable!(
                    "static emptys/repeats/seqs/pars shouldn't be in domination map"
                ),
            },
        }
        reads
    }
}

///Contains the name of a cell. The methods in this struct are used to search to
///see if there was a write to the cell name given a set of nodes.
pub struct NodeSearch {
    name: ir::Id,
}

impl NodeSearch {
    pub fn new(name: ir::Id) -> Self {
        NodeSearch { name }
    }

    // Given a vec of assignments, return true if the go port of self.name
    // is guaranteed to be written to in assignments. By "guarantee" we mean
    // the guard is true and the src is constant(1,1).
    fn go_is_written<T>(&self, assignments: &[ir::Assignment<T>]) -> bool {
        assignments.iter().any(|assign: &ir::Assignment<T>| {
            let dst_ref = assign.dst.borrow();
            if dst_ref.attributes.has(ir::NumAttr::Go)
                && assign.guard.is_true()
                && assign.src.borrow().is_constant(1, 1)
            {
                if let ir::PortParent::Cell(cell_wref) = &dst_ref.parent {
                    return cell_wref.upgrade().borrow().name() == self.name;
                }
            }
            false
        })
    }

    // returns true if outputs or comp indicates that cell named self.name was
    // written to, false otherwise
    fn is_written_invoke(
        &self,
        outputs: &[(ir::Id, ir::RRC<ir::Port>)],
        comp: &ir::RRC<ir::Cell>,
    ) -> bool {
        for (_, port) in outputs.iter() {
            if port.borrow().get_parent_name() == self.name {
                return true;
            }
        }
        if comp.borrow().name() == self.name {
            return true;
        }
        false
    }

    //Returns true if any of the control statements in dominators write to a cell
    //with self's name.
    pub fn is_written_guaranteed(
        &self,
        dominators: &HashSet<u64>,
        comp: &mut ir::Component,
    ) -> bool {
        let main_control = comp.control.borrow();
        let (dominator_controls, dominator_static_controls) =
            DominatorMap::get_control_nodes(dominators, &main_control);
        for c in dominator_controls {
            match c {
                ir::Control::Empty(_)
                | ir::Control::Seq(_)
                | ir::Control::Par(_)
                | ir::Control::Repeat(_)
                | ir::Control::Static(_) => {
                    unreachable!(
                        "no empty/seqs/pars/repeat/static should be in domination map"
                    )
                }
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    if self.go_is_written(&group.borrow().assignments) {
                        return true;
                    }
                }
                //You can't have a write to a stateful component in a
                //combinational group.
                ir::Control::While(_) | ir::Control::If(_) => (),
                ir::Control::Invoke(ir::Invoke { comp, outputs, .. }) => {
                    if self.is_written_invoke(outputs, comp) {
                        return true;
                    }
                }
            }
        }
        for sc in dominator_static_controls {
            match sc {
                ir::StaticControl::Empty(_)
                | ir::StaticControl::Seq(_)
                | ir::StaticControl::Par(_)
                | ir::StaticControl::Repeat(_) => unreachable!(
                    "no static repeats/seqs/pars should be in domination map"
                ),
                ir::StaticControl::Invoke(ir::StaticInvoke {
                    comp,
                    outputs,
                    ..
                }) => {
                    if self.is_written_invoke(outputs, comp) {
                        return true;
                    }
                }
                ir::StaticControl::Enable(ir::StaticEnable {
                    group, ..
                }) => {
                    if self.go_is_written(&group.borrow().assignments) {
                        return true;
                    }
                }
                // "if nodes" (which are really just the guard) do not write to components
                // therefore, we should return false
                ir::StaticControl::If(_) => (),
            }
        }
        false
    }
}
