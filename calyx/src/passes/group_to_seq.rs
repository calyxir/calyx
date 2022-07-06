use crate::analysis::{ReadWriteSet, ShareSet};
use crate::ir::{traversal::VisResult, RRC};

#[derive(Default)]
/// Description goes here

pub struct GroupToSeq {
    go_done_map: HashMap<(ir::Id, ir::Id)>,
    cells_of_interest: HashSet<ir::Id>,
    last: Option<ir::Id>,
    share: ShareSet,
    state_share: ShareSet,
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
        let state_share = ShareSet::from_context::<true>(ctx);
        let share = ShareSet::from_context::<false>(ctx);

        Ok(GroupToSeq {
            go_done_map: HashMap::new(),
            cells_of_interest: HashSet::new(),
            last: None,
            state_shareable,
            shareable,
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
        match cell.borrow().prototype {
            ir::CellType::Primitive { name, .. }
            | ir::CellType::Component { name } => {
                !self.shareable.contains(name)
            }
            _ => false,
        }
    }
    fn cells_of_iterest(&mut self, asmts: &Vec) {
        self.cells_of_interest = ReadWriteSet::uses(asmts)
            .filter(|cell| self.is_stateful(cell))
            .map(|cell| cell.borrow().name().clone())
            .collect()
    }
    fn is_cell_of_interest(&self, cell: &ir::RRC<ir::Cell>) -> bool {
        self.cells_of_interest.contains(cell.borrow().name())
    }
    fn update(&mut self, src: &ir::Port, dst: &ir::Port) -> bool {
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
                        .get(src_cell.upgrade().borrow().clone_name())
                    {
                        None => {
                            self.go_done_map.insert(
                                src_cell.upgrade().borrow().clone_name(),
                                dst_cell.upgrade().borrow().clone_name(),
                            );
                        }
                        Some(name) => {
                            if name != dst_cell.upgrade().borrow().clone_name()
                            {
                                return false;
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
                    self.last = Some(src_cell.upgrade().clone_name());
                }
            }
            // If we encounter anything else not of interest to us
            _ => (),
        }
        true
    }
    fn get_connector(&self, name: &ir::Id) -> Option<ir::Id> {
        if let Some((dst, _)) =
            self.go_done_map.iter().find(|(_, src)| src == name)
        {
            Some(dst)
        } else {
            None
        }
    }
}

impl Visitor for GroupToSeq {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let asmts = s.group.borrow().assignments;
        self.cells_of_interest(asmts);
        for asmt in asmts {
            let src = asmt.src;
            let dst = asmt.dst;
            if !self.update(&src, &dst) {
                return Ok(Action::Continue);
            }
        }
        let mut ordering: Vec<ir::Id> = Vec::new();
        if let Some(last_cell) = self.last {
            vec.push(last_cell);
        } else {
            return Ok(Action::Continue);
        }
    }
}
