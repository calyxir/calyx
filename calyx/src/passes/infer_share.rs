use crate::analysis::{DominatorMap, ReadWriteSet, ShareSet};
use crate::errors::CalyxResult;
use crate::ir;
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use crate::ir::CloneName;
use std::collections::HashSet;
use std::rc::Rc;

const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

//given an RRC of a cell, determines if cell is a std_mem_cell. There may be an
//easier way to do this.
fn is_std_mem(cell: &ir::Cell) -> bool {
    match &cell.prototype {
        ir::CellType::Primitive { name, .. } => {
            let cell_name = name.id.clone();
            cell_name == "std_mem_d1"
                || cell_name == "std_mem_d2"
                || cell_name == "std_mem_d3"
                || cell_name == "std_mem_d4"
        }
        _ => false,
    }
}

//Inputs are a control statement c and a u64 id. If control is an if statment, then
//the id should refer to either the begin or end id of stmt c. Returns true if id refers
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

pub struct InferShare {
    print_dmap: bool,
    state_shareable: ShareSet,
    //nonshareable component names
    no_share: HashSet<ir::Id>,
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
        let opts = Self::get_opts(&["print_dmap"], ctx);

        let state_shareable = ShareSet::from_context::<true>(ctx);

        Ok(InferShare {
            print_dmap: opts[0],
            state_shareable,
            no_share: HashSet::new(),
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

        //cannot contain any ref cells, non-shareable components, or mem cells
        if comp.cells.iter().any(|cell| {
            let cell_ref = cell.borrow();
            cell_ref.is_reference()
                || match cell_ref.type_name() {
                    Some(name) => self.no_share.contains(name),
                    None => false,
                }
                || is_std_mem(&cell_ref)
        }) {
            self.no_share.insert(comp.name.clone());
            return Ok(Action::Continue);
        }

        //build the domination map
        let mut dmap = DominatorMap::new(&mut comp.control.borrow_mut(), &comp);
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
                    self.no_share.insert(comp.name.clone());
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
            && Self::guard_only_dones(&assignment.guard)
    }

    //returns true if port is a "done" port or is a constant
    fn is_done_port(port: &ir::RRC<ir::Port>) -> bool {
        port.borrow().name.id == "done" || port.borrow().is_constant(1, 1)
    }

    //returns true if guard only contains dones, or is true
    fn guard_only_dones(guard: &ir::Guard) -> bool {
        match guard {
            ir::Guard::Or(g1, g2) | ir::Guard::And(g1, g2) => {
                Self::guard_only_dones(g1) && Self::guard_only_dones(g2)
            }
            ir::Guard::Not(g) => Self::guard_only_dones(g),
            ir::Guard::True => true,
            ir::Guard::CompOp(_, p1, p2) => {
                Self::is_done_port(p1) && Self::is_done_port(p2)
            }
            ir::Guard::Port(p) => Self::is_done_port(p),
        }
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
            ir::Control::Enable(ir::Enable { group, .. }) => {
                self.add_assignment_reads(
                    state_shareable,
                    &group.borrow().assignments,
                );
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
            ir::Control::Invoke(ir::Invoke {
                comp,
                inputs,
                outputs,
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

    //returns true if the port's parent's name matches name, false otherwise
    fn parent_matches_name(&self, port: &ir::RRC<ir::Port>) -> bool {
        if let ir::PortParent::Cell(cell) = &port.borrow().parent {
            if cell.upgrade().borrow().name().clone() == self.name {
                return true;
            }
        }
        false
    }

    //returns Some(cell) if the assignment is to cell's go port and is guaranteed
    //to be activated. To check this guarantee, we check if the assignment guard
    //is true and assignment src is 1'd1.
    fn guaranteed_go(assign: &ir::Assignment) -> Option<ir::RRC<ir::Cell>> {
        let dst_ref = assign.dst.borrow();
        if dst_ref.attributes.has("go")
            && Self::guard_true(&assign.guard)
            && Self::src_const(&assign.src)
        {
            if let ir::PortParent::Cell(cell_wref) = &dst_ref.parent {
                return Some(Rc::clone(&cell_wref.upgrade()));
            }
        }
        None
    }

    //returns true if port is 1'd1
    fn src_const(port: &ir::RRC<ir::Port>) -> bool {
        port.borrow().is_constant(1, 1)
    }

    //returns true if guard is True
    fn guard_true(guard: &ir::Guard) -> bool {
        matches!(guard, ir::Guard::True)
    }

    //given a vec of assignments, return true if name is ever written to in
    //assignments, false otherwise
    fn search_assignments(&self, assignments: &[ir::Assignment]) -> bool {
        for write in assignments.iter().filter_map(Self::guaranteed_go) {
            if *write.borrow().name() == self.name {
                return true;
            }
        }
        false
    }

    //Returns true if any of the control statements in dominators write to a cell
    //with self's name.
    fn is_written(
        &self,
        dominators: &HashSet<u64>,
        comp: &mut ir::Component,
        shareable: &ShareSet,
    ) -> bool {
        for dominator in dominators {
            if let Some(c) =
                DominatorMap::get_control(*dominator, &comp.control.borrow())
            {
                match c {
                    ir::Control::Empty(_)
                    | ir::Control::Seq(_)
                    | ir::Control::Par(_) => {
                        unreachable!(
                            "no empty/seqs/pars should be in domination map"
                        )
                    }
                    ir::Control::Enable(ir::Enable { group, .. }) => {
                        if self.search_assignments(&group.borrow().assignments)
                        {
                            return true;
                        }
                    }
                    //You can't have a write to a stateful component in a
                    //combinational group.
                    ir::Control::While(_) | ir::Control::If(_) => (),
                    ir::Control::Invoke(ir::Invoke {
                        comp,
                        inputs,
                        outputs,
                        ..
                    }) => {
                        for (_, port) in outputs.iter() {
                            if self.parent_matches_name(port) {
                                return true;
                            }
                        }
                        if shareable.is_shareable_component(comp)
                            && !inputs.is_empty()
                        {
                            return *comp.borrow().name() == self.name;
                        }
                    }
                }
            } else {
                unreachable!("should always be able to get control from id")
            }
        }
        false
    }
}
