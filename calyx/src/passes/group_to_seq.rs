use crate::analysis::{ReadWriteSet, ShareSet};
use crate::errors::CalyxResult;
use crate::ir;
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::rc::Rc;

/// Description goes here
pub struct GroupToSeq {
    go_done_map: HashMap<ir::Id, ir::Id>,
    cells_of_interest: HashSet<ir::Id>,
    last: Option<ir::Id>,
    share: ShareSet,
}

enum WriteType {
    Cell(ir::Id),
    GroupDone,
    Other,
}

impl Named for GroupToSeq {
    fn name() -> &'static str {
        "group-to-seq"
    }

    fn description() -> &'static str {
        "split groups under correct conditions"
    }
}

impl ConstructVisitor for GroupToSeq {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let share = ShareSet::from_context::<false>(ctx);

        Ok(GroupToSeq {
            go_done_map: HashMap::new(),
            cells_of_interest: HashSet::new(),
            last: None,
            share,
        })
    }

    fn clear_data(&mut self) {
        self.go_done_map = HashMap::new();
        self.cells_of_interest = HashSet::new();
        self.last = None;
    }
}

impl GroupToSeq {
    fn is_stateful(&self, cell: &ir::RRC<ir::Cell>) -> bool {
        match &cell.borrow().prototype {
            ir::CellType::Primitive { name, .. }
            | ir::CellType::Component { name } => !self.share.contains(&name),
            _ => false,
        }
    }
    fn get_cells_of_interest(&mut self, asmts: &Vec<ir::Assignment>) {
        self.cells_of_interest = ReadWriteSet::write_set(asmts.iter())
            .filter(|cell| self.is_stateful(cell))
            .map(|cell| cell.borrow().name().clone())
            .collect()
    }
    fn is_cell_of_interest(&self, cell: &ir::RRC<ir::Cell>) -> bool {
        self.cells_of_interest.contains(cell.borrow().name())
    }
    fn is_go_done(&self, asmt: &ir::Assignment) -> bool {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            //src's done writes to dst's go
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
                if self.is_cell_of_interest(&src_cell.upgrade())
                    && self.is_cell_of_interest(&dst_cell.upgrade())
                    && src.name == "done"
                    && dst.attributes.has("go")
                {
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    fn update(&mut self, asmt: &ir::Assignment) -> (bool, bool) {
        let src = asmt.src.borrow();
        let dst = asmt.dst.borrow();
        match (&src.parent, &dst.parent) {
            //src's done writes to dst's go
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
                if self.is_cell_of_interest(&src_cell.upgrade())
                    && self.is_cell_of_interest(&dst_cell.upgrade())
                    && src.name == "done"
                    && dst.attributes.has("go")
                {
                    match self
                        .go_done_map
                        .get(src_cell.upgrade().borrow().name())
                    {
                        None => {
                            self.go_done_map.insert(
                                src_cell.upgrade().borrow().name().clone(),
                                dst_cell.upgrade().borrow().name().clone(),
                            );
                            return (true, true);
                        }
                        Some(name) => {
                            if name != dst_cell.upgrade().borrow().name() {
                                return (false, false);
                            }
                        }
                    }
                }
            }
            // src_cell's done writes to group's done
            (ir::PortParent::Cell(src_cell), ir::PortParent::Group(_)) => {
                if dst.name == "done"
                    && self.is_cell_of_interest(&src_cell.upgrade())
                    && src.name == "done"
                {
                    self.last =
                        Some(src_cell.upgrade().borrow().name().clone());
                    return (true, false);
                }
            }
            // If we encounter anything else not of interest to us
            _ => (),
        }
        (true, false)
    }
    fn get_connector(&self, name: &ir::Id) -> Option<ir::Id> {
        if let Some((dst, _)) =
            self.go_done_map.iter().find(|(_, src)| *src == name)
        {
            Some(dst.clone())
        } else {
            None
        }
    }
    fn is_write_to(&self, asmt: &ir::Assignment) -> WriteType {
        match &asmt.dst.borrow().parent {
            ir::PortParent::Cell(cell) => {
                WriteType::Cell(cell.upgrade().borrow().name().clone())
            }
            ir::PortParent::Group(_) => {
                if &asmt.dst.borrow().name == "done" {
                    WriteType::GroupDone
                } else {
                    WriteType::Other
                }
            }
        }
    }

    fn is_valid_assignment(&self, asmt: &ir::Assignment) -> bool {
        match &asmt.dst.borrow().parent {
            ir::PortParent::Cell(cell) => {
                if self.is_cell_of_interest(&cell.upgrade()) {
                    true
                } else {
                    false
                }
            }
            ir::PortParent::Group(_) => asmt.dst.borrow().name == "done",
        }
    }

    fn split_go_done(
        builder: &mut ir::Builder,
        asmt: ir::Assignment,
        group_wref: ir::WRC<ir::Group>,
    ) -> (ir::Assignment, ir::Assignment) {
        let con = builder.add_constant(1, 1);
        let src = ir::Port {
            name: ir::Id::new("const 1", None),
            width: 1,
            direction: ir::Direction::Output,
            parent: ir::PortParent::Cell(ir::WRC::from(&con)),
            attributes: ir::Attributes::default(),
        };
        let src_ref = Rc::new(RefCell::new(src));

        let dst = ir::Port {
            name: ir::Id::new("done", None),
            width: 1,
            direction: ir::Direction::Input,
            parent: ir::PortParent::Group(group_wref),
            attributes: ir::Attributes::default(),
        };
        let dst_ref = Rc::new(RefCell::new(dst));
        (
            builder.build_assignment(dst_ref, asmt.src, ir::Guard::True),
            builder.build_assignment(asmt.dst, src_ref, ir::Guard::True),
        )
    }

    fn rename_group_done(
        builder: &mut ir::Builder,
        asmt: ir::Assignment,
        group: ir::WRC<ir::Group>,
    ) -> ir::Assignment {
        let dst = ir::Port {
            name: ir::Id::new("done", None),
            width: 1,
            direction: ir::Direction::Input,
            parent: ir::PortParent::Group(group),
            attributes: ir::Attributes::default(),
        };
        let dst_ref = Rc::new(RefCell::new(dst));
        builder.build_assignment(dst_ref, asmt.src, ir::Guard::True)
    }
}

fn get_src_name(asmt: &ir::Assignment) -> ir::Id {
    match &asmt.src.borrow().parent {
        ir::PortParent::Cell(cell) => cell.upgrade().borrow().name().clone(),
        ir::PortParent::Group(group) => group.upgrade().borrow().name().clone(),
    }
}
impl Visitor for GroupToSeq {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        env::set_var("RUST_BACKTRACE", "1");
        let mut builder = ir::Builder::new(comp, sigs);
        let mut group = s.group.borrow_mut();
        self.get_cells_of_interest(&group.assignments);
        for asmt in &group.assignments {
            let (b1, _) = self.update(asmt);
            if !b1 {
                return Ok(Action::Continue);
            }
        }
        let mut ordering: Vec<ir::Id> = Vec::new();
        if let Some(last_cell) = self.last.clone() {
            ordering.push(last_cell.clone());
            let mut cur = last_cell.clone();
            while let Some(new_cell) = self.get_connector(&cur) {
                ordering.insert(0, new_cell.clone());
                cur = new_cell.clone();
            }
        } else {
            return Ok(Action::Continue);
        }
        if !group
            .assignments
            .iter()
            .all(|asmt| self.is_valid_assignment(asmt))
        {
            return Ok(Action::Continue);
        }
        if ordering.len() > 1 && ordering.len() == self.cells_of_interest.len()
        {
            let mut model: BTreeMap<ir::Id, Vec<ir::Assignment>> =
                BTreeMap::new();
            let mut go_done_asmts: BTreeMap<ir::Id, ir::Assignment> =
                BTreeMap::new();
            let mut group_done: Vec<ir::Assignment> = Vec::new();

            ordering.iter().for_each(|cell| {
                model.insert(cell.clone(), Vec::new());
            });

            for asmt in group.assignments.drain(..) {
                match self.is_write_to(&asmt) {
                    WriteType::Cell(cell_name) => {
                        if self.is_go_done(&asmt) {
                            go_done_asmts.insert(get_src_name(&asmt), asmt);
                        } else if let Some(new_asmts) =
                            model.get_mut(&cell_name)
                        {
                            new_asmts.push(asmt);
                        } else {
                            unreachable!("shouldn't ever occur")
                        }
                    }
                    WriteType::GroupDone => group_done.push(asmt),
                    WriteType::Other => unreachable!("shouldn't occur"),
                }
            }

            let mut seq_vec: Vec<ir::Control> = Vec::new();
            let mut begin_asmt: Vec<ir::Assignment> = Vec::new();

            for cell in ordering {
                if let Some(asmts) = model.remove(&cell) {
                    let group = builder.add_group("TODO");
                    if begin_asmt.len() > 0 {
                        group
                            .borrow_mut()
                            .assignments
                            .push(begin_asmt.remove(0));
                    }
                    group.borrow_mut().assignments.extend(asmts.into_iter());
                    if let Some(asmt) = go_done_asmts.remove(&cell) {
                        let (group_done, cell_go) = GroupToSeq::split_go_done(
                            &mut builder,
                            asmt,
                            ir::WRC::from(&group),
                        );
                        group.borrow_mut().assignments.push(group_done);
                        begin_asmt.push(cell_go);
                    } else {
                        if group_done.len() == 1 {
                            let new_assign = GroupToSeq::rename_group_done(
                                &mut builder,
                                group_done.remove(0),
                                ir::WRC::from(&group),
                            );
                            group.borrow_mut().assignments.push(new_assign);
                        } else {
                            unreachable!("shouldn't happen")
                        }
                    }
                    seq_vec.push(ir::Control::enable(group));
                } else {
                    unreachable!("each cell in ordering should be in model")
                }
            }
            Ok(Action::Change(Box::new(ir::Control::seq(seq_vec))))
        } else {
            Ok(Action::Continue)
        }
    }
}
