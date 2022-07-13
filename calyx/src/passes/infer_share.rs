use crate::analysis::{DominatorMap, ReadWriteSet, ShareSet};
use crate::errors::CalyxResult;
use crate::ir;
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use crate::ir::CloneName;
use std::collections::HashSet;

const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

// Inputs are a control statement c and a u64 id. If control is an if statment, then
// the id should refer to either the begin or end id of stmt c. Returns true if id refers
// to the begin id and false if it refers to the end id. If it is not an if statement, behavior
// is unspecified.
fn is_begin_id(c: &ir::Control, id: u64) -> bool {
    match c {
        ir::Control::If(if_control) => {
            if let Some(&begin) = if_control.attributes.get(BEGIN_ID) {
                if begin == id {
                    return true;
                }
            }
            if let Some(&end) = if_control.attributes.get(END_ID) {
                if end == id {
                    return false;
                }
            }
            unreachable!("id should match either beginning or ending id")
        }
        _ => true,
    }
}

/// This pass checks if components are (state) shareable. Here is the process it
/// goes through. If a component uses any ref cells, or non-shareable cells then it
/// is automatically not shareable. Otherwise, check if each read of a stateful
/// cell is guaranteed to be dominated by a write. We check this
/// by building a domination map.
pub struct InferShare {
    print_dmap: bool,
    state_shareable: ShareSet,
    shareable: ShareSet,
    //name of main (so we can skip it)
    main: ir::Id,
}

impl Named for InferShare {
    fn name() -> &'static str {
        "infer-share"
    }

    fn description() -> &'static str {
        "Infer User Defined Components as Shareable"
    }
}

impl ConstructVisitor for InferShare {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(&["print-dmap"], ctx);

        let state_shareable = ShareSet::from_context::<true>(ctx);
        let shareable = ShareSet::from_context::<false>(ctx);

        Ok(InferShare {
            print_dmap: opts[0],
            state_shareable,
            shareable,
            main: ctx.entrypoint.clone(),
        })
    }

    fn clear_data(&mut self) {}
}

impl Visitor for InferShare {
    fn require_postorder() -> bool {
        true
    }
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        //if the component is main, then we can stop checking
        if comp.name == self.main {
            return Ok(Action::Continue);
        }

        //cell is type ThisComponent or Constant
        let const_or_this = |cell: &ir::RRC<ir::Cell>| -> bool {
            match cell.borrow().prototype {
                ir::CellType::ThisComponent | ir::CellType::Constant { .. } => {
                    true
                }
                _ => false,
            }
        };

        //cell is shareble, state_shareable, const, or This component
        let type_is_shareable = |cell: &ir::RRC<ir::Cell>| -> bool {
            const_or_this(cell)
                || self.shareable.is_shareable_component(cell)
                || self.state_shareable.is_shareable_component(cell)
        };

        //cannot contain any ref cells, or non shareable/stateshareable cells.
        if comp.cells.iter().any(|cell| {
            cell.borrow().is_reference() || !type_is_shareable(cell)
        }) {
            return Ok(Action::Continue);
        }

        //build the domination map
        let mut dmap = DominatorMap::new(
            &mut comp.control.borrow_mut(),
            comp.name.id.clone(),
        );
        if self.print_dmap {
            println!("{dmap:?}");
        }

        for (node, dominators) in dmap.map.iter_mut() {
            //get the reads
            let mut reads: ReadSet = ReadSet::default();
            if let Some(c) =
                DominatorMap::get_control(*node, &comp.control.borrow())
            {
                reads.get_reads_from_control(
                    c,
                    &self.state_shareable,
                    is_begin_id(c, *node),
                );
            }

            //if read and write occur in same group/invoke, then we cannot label it
            //shareable. So we remove node from its dominators
            dominators.remove(node);
            for cell_name in reads.reads.clone() {
                let key = NameSearch::new(cell_name);
                if !key.is_written(dominators, comp, &self.state_shareable) {
                    return Ok(Action::Continue);
                }
            }
        }
        comp.attributes.insert("state_share", 1);
        self.state_shareable.add(comp.name.clone());
        Ok(Action::Continue)
    }
}

///Contains the ids of all the cells that are read from in a given "node" in
///the domination map graph
#[derive(Default)]
struct ReadSet {
    pub reads: HashSet<ir::Id>,
}
impl ReadSet {
    //given a port, insert the port's parent's id if the parent the port's parent
    //is shareable
    fn add_parent_if_shareable(
        &mut self,
        share: &ShareSet,
        port: &ir::RRC<ir::Port>,
    ) {
        if let ir::PortParent::Cell(cell) = &port.borrow().parent {
            if share.is_shareable_component(&cell.upgrade()) {
                self.reads.insert(cell.upgrade().borrow().name().clone());
            }
        }
    }

    //if the assignment reads only dones, return true. This is used so that we
    //can ignore reads of "done" cells.
    fn reads_only_dones(assignment: &ir::Assignment) -> bool {
        Self::is_done_port(&assignment.src)
            && assignment
                .guard
                .all_ports()
                .iter()
                .all(|port: &ir::RRC<ir::Port>| Self::is_done_port(port))
    }

    //returns true if port is a "done" port or is a constant
    fn is_done_port(port: &ir::RRC<ir::Port>) -> bool {
        port.borrow().attributes.has("done") || port.borrow().is_constant(1, 1)
    }

    //Adds the ids of any state_shareable cells that are read from in assignments,
    //excluding reads where the only reads are from "done" ports.
    fn add_assignment_reads(
        &mut self,
        share: &ShareSet,
        assignments: &[ir::Assignment],
    ) {
        for cell in ReadWriteSet::read_set(
            assignments
                .iter()
                .filter(|assign| !Self::reads_only_dones(assign)),
        ) {
            if share.is_shareable_component(&cell) {
                self.reads.insert(cell.borrow().name().clone());
            }
        }
    }

    //Given a control statement c, adds all of the reads of shareable cells from c.
    //For while loops and if stmts, the control refers only to the guard, not the body.
    fn get_reads_from_control(
        &mut self,
        c: &ir::Control,
        state_shareable: &ShareSet,
        is_begin_id: bool,
    ) {
        match c {
            ir::Control::Empty(_)
            | ir::Control::Seq(_)
            | ir::Control::Par(_) => {
                unreachable!("no empty/seqs/pars should be in domination map")
            }
            ir::Control::If(ir::If { port, cond, .. }) => {
                if is_begin_id {
                    self.add_parent_if_shareable(state_shareable, port);
                    if let Some(group) = cond {
                        self.add_assignment_reads(
                            state_shareable,
                            &group.borrow().assignments,
                        );
                    }
                }
            }
            ir::Control::While(ir::While { port, cond, .. }) => {
                self.add_parent_if_shareable(state_shareable, port);
                if let Some(group) = cond {
                    self.add_assignment_reads(
                        state_shareable,
                        &group.borrow().assignments,
                    );
                }
            }
            ir::Control::Enable(ir::Enable { group, .. }) => {
                self.add_assignment_reads(
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
                    self.add_parent_if_shareable(state_shareable, port);
                }
                if !outputs.is_empty()
                    && state_shareable.is_shareable_component(comp)
                {
                    self.reads.insert(comp.clone_name());
                }
                if let Some(group) = comb_group {
                    self.add_assignment_reads(
                        state_shareable,
                        &group.borrow().assignments,
                    );
                }
            }
        }
    }
}

///Contains the name of a cell. The methods in this struct are used to search to
///see if there was a write to the cell name.
struct NameSearch {
    name: ir::Id,
}

impl NameSearch {
    fn new(name: ir::Id) -> Self {
        NameSearch { name }
    }

    // Given a vec of assignments, return true if the go port of self.name
    // is guaranteed to be written to in assignments. By "guarantee" we mean
    // the guard is true and the src is constant(1,1).
    fn go_is_written(&self, assignments: &[ir::Assignment]) -> bool {
        assignments.iter().any(|assign: &ir::Assignment| {
            let dst_ref = assign.dst.borrow();
            if dst_ref.attributes.has("go")
                && assign.guard.is_true()
                && assign.src.borrow().is_constant(1, 1)
            {
                if let ir::PortParent::Cell(cell_wref) = &dst_ref.parent {
                    return *cell_wref.upgrade().borrow().name() == self.name;
                }
            }
            false
        })
    }

    //Returns true if any of the control statements in dominators write to a cell
    //with self's name.
    fn is_written(
        &self,
        dominators: &HashSet<u64>,
        comp: &mut ir::Component,
        shareable: &ShareSet,
    ) -> bool {
        let main_control = comp.control.borrow();
        let dominator_controls =
            DominatorMap::get_control_nodes(dominators, &main_control);
        for c in dominator_controls {
            match c {
                ir::Control::Empty(_)
                | ir::Control::Seq(_)
                | ir::Control::Par(_) => {
                    unreachable!(
                        "no empty/seqs/pars should be in domination map"
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
                    for (_, port) in outputs.iter() {
                        if port.borrow().get_parent_name() == self.name {
                            return true;
                        }
                    }
                    if shareable.is_shareable_component(comp) {
                        if *comp.borrow().name() == self.name {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
