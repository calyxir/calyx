use crate::analysis::{DominatorMap, ReadWriteSet};
use crate::errors::CalyxResult;
use crate::ir;
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use std::collections::HashSet;

const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

#[derive(Default, Clone)]
pub struct ShareSet {
    shareable: HashSet<ir::Id>,
}

impl ShareSet {
    fn new(set: HashSet<ir::Id>) -> ShareSet {
        ShareSet { shareable: set }
    }
    //given a set of shareable and a cell, determines whether cell's
    //type is shareable or not
    pub fn is_shareable_component(&self, cell: &ir::RRC<ir::Cell>) -> bool {
        if let Some(type_name) = cell.borrow().type_name() {
            self.shareable.contains(type_name)
        } else {
            false
        }
    }
}

#[derive(Default)]
/// Description goes here
pub struct InferShare {
    /// Set of state shareable components (as type names)
    state_shareable: ShareSet,
}

/*impl ConstructVisitor for InferShare {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let mut state_shareable = HashSet::new();
        let mut shareable = HashSet::new();
        // add state_share=1 primitives to the state_shareable set
        for prim in ctx.lib.signatures() {
            if prim.attributes.has("state_share") {
                state_shareable.insert(prim.name.clone());
            }
        }
        let infer_share = InferShare {
            state_shareable: ShareSet::new(state_shareable),
        };
        Ok(infer_share)
    }

    fn clear_data(&mut self) {}
}*/

impl Named for InferShare {
    fn name() -> &'static str {
        "infer-share"
    }

    fn description() -> &'static str {
        "Infer User Defined Components as Shareable"
    }
}

fn add_parent_if_shareable(
    share: &ShareSet,
    port: &ir::RRC<ir::Port>,
    share_reads: &mut HashSet<ir::Id>,
) {
    if let ir::PortParent::Cell(cell) = &port.borrow().parent {
        if share.is_shareable_component(&cell.upgrade()) {
            share_reads.insert(cell.upgrade().borrow().name().clone());
        }
    }
}

fn add_group_reads(
    share: &ShareSet,
    group: &ir::RRC<ir::Group>,
    share_reads: &mut HashSet<ir::Id>,
) {
    for cell in ReadWriteSet::read_set(group.borrow().assignments.iter()) {
        if share.is_shareable_component(&cell) {
            share_reads.insert(cell.borrow().name().clone());
        }
    }
}

fn add_comb_group_reads(
    share: &ShareSet,
    group: &ir::RRC<ir::CombGroup>,
    share_reads: &mut HashSet<ir::Id>,
) {
    for cell in ReadWriteSet::read_set(group.borrow().assignments.iter()) {
        if share.is_shareable_component(&cell) {
            share_reads.insert(cell.borrow().name().clone());
        }
    }
}

fn is_begin_id(c: &ir::Control, id: u64) -> bool {
    match c {
        ir::Control::If(if_control) => {
            if let Some(&begin) = if_control.attributes.get(BEGIN_ID) {
                if begin == id {
                    return true;
                }
            } else if let Some(&end) = if_control.attributes.get(END_ID) {
                if end == id {
                    return false;
                }
            }
            unreachable!("id should match either beginning or ending id")
        }
        _ => unreachable!("only call on if stmts"),
    }
}

fn parent_matches_name(port: &ir::RRC<ir::Port>, name: &ir::Id) -> bool {
    if let ir::PortParent::Cell(cell) = &port.borrow().parent {
        if cell.upgrade().borrow().name().clone() == name {
            return true;
        }
    }
    false
}

fn look_for_writes(assignments: &Vec<ir::Assignment>, name: &ir::Id) -> bool {
    //Does this gaurantee read/write stuff?
    for write in ReadWriteSet::write_set(assignments.iter()) {
        if write.borrow().name() == name {
            return true;
        }
    }
    false
}

fn find_write(
    dominators: &HashSet<u64>,
    name: &ir::Id,
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
                | ir::Control::Par(_) => unreachable!(
                    "no empty/seqs/pars should be in domination map"
                ),
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    if look_for_writes(&group.borrow().assignments, name) {
                        return true;
                    }
                }
                ir::Control::While(_) | ir::Control::If(_) => (),
                ir::Control::Invoke(ir::Invoke {
                    comp,
                    inputs,
                    outputs,
                    ..
                }) => {
                    for (_, port) in outputs.iter() {
                        if parent_matches_name(&port, name) {
                            return true;
                        }
                    }
                    if shareable.is_shareable_component(comp) {
                        return true;
                    }
                }
            }
        } else {
            return false;
        }
    }
    false
}

impl Visitor for InferShare {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        //FIRST CHECK IF COMP HAS ANY NON-SHAREABLE COMPONENTS
        let mut dmap = DominatorMap::new(&mut comp.control.borrow_mut());
        for (node, dominators) in dmap.map.iter() {
            let mut share_reads: HashSet<ir::Id> = HashSet::new();
            if let Some(c) =
                DominatorMap::get_control(*node, &comp.control.borrow())
            {
                match c {
                    ir::Control::Empty(_)
                    | ir::Control::Seq(_)
                    | ir::Control::Par(_) => unreachable!(
                        "no empty/seqs/pars should be in domination map"
                    ),
                    ir::Control::Enable(ir::Enable { group, .. }) => {
                        add_group_reads(
                            &self.state_shareable,
                            &group,
                            &mut share_reads,
                        );
                    }
                    ir::Control::While(ir::While { port, cond, .. }) => {
                        add_parent_if_shareable(
                            &self.state_shareable,
                            &port,
                            &mut share_reads,
                        );
                        if let Some(group) = cond {
                            add_comb_group_reads(
                                &self.state_shareable,
                                group,
                                &mut share_reads,
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
                            add_parent_if_shareable(
                                &self.state_shareable,
                                &port,
                                &mut share_reads,
                            );
                        }
                    }
                    ir::Control::If(ir::If { port, cond, .. }) => {
                        if is_begin_id(c, *node) {
                            add_parent_if_shareable(
                                &self.state_shareable,
                                &port,
                                &mut share_reads,
                            );
                            if let Some(group) = cond {
                                add_comb_group_reads(
                                    &self.state_shareable,
                                    group,
                                    &mut share_reads,
                                );
                            }
                        }
                    }
                }
            }
            for cell_name in share_reads {
                if !find_write(
                    dominators,
                    &cell_name,
                    comp,
                    &self.state_shareable,
                ) {
                    //non shareable
                    comp.attributes.insert("non_share", 1);
                }
            }
            //shareable
            comp.attributes.insert("state_share", 1);
        }
        Ok(Action::Continue)
    }
}
