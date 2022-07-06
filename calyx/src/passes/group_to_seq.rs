use crate::analysis::{ReadWriteSet, ShareSet};
use crate::ir::RRC;

#[derive(Default)]
/// Description goes here

pub struct GroupToSeq {
    go_done_map: HashMap<(ir::Id, ir::Id)>,
    go_constants: HashSet<(ir::Id)>,
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
            go_constants: HashSet::new(),
            last: None,
            state_shareable,
            shareable,
        })
    }

    fn clear_data(&mut self) {
        self.go_done_map = HashMap::new();
        self.go_constants = HashSet::new();
        self.last = None;
    }
}

impl GroupToSeq {
    fn update(&mut self, src: &ir::Port, dst: &ir::Port) -> bool {
        match (&src.parent, &dst.parent) {
            (
                ir::PortParent::Cell(src_cell),
                ir::PortParent::Cell(dst_cell),
            ) => {
                match (
                    &dst_cell.upgrade().borrow().prototype,
                    &src_cell.upgrade().borrow().prototype,
                ) {
                    (
                        ir::CellType::Primitive {
                            name: dst_cell_prim_type,
                            ..
                        },
                        ir::CellType::Primitive {
                            name: src_cell_prim_type,
                            ..
                        },
                    )
                    | (
                        ir::CellType::Primitive {
                            name: dst_cell_prim_type,
                            ..
                        },
                        ir::CellType::Component {
                            name: src_cell_prim_type,
                            ..
                        },
                    )
                    | (
                        ir::CellType::Component {
                            name: dst_cell_prim_type,
                            ..
                        },
                        ir::CellType::Primitive {
                            name: src_cell_prim_type,
                            ..
                        },
                    )
                    | (
                        ir::CellType::Component {
                            name: dst_cell_prim_type,
                            ..
                        },
                        ir::CellType::Component {
                            name: src_cell_prim_type,
                            ..
                        },
                    ) => {
                        if !(self.shareable.contains(dst_cell_prim_type)
                            || self.shareable.contains(src_cell_prim_type))
                            && (dst.attributes.has("go")
                                && src.clone_name() == "done")
                        {
                            self.go_done_map.insert(
                                dst_cell.upgrade().borrow().clone_name(),
                                src_cell.upgrade().borrow().clone_name(),
                            );
                        }
                    }
                    (
                        ir::CellType::Component {
                            name: dst_cell_prim_type,
                            ..
                        },
                        ir::CellType::Constant { .. },
                    )
                    | (
                        ir::CellType::Primitive {
                            name: dst_cell_prim_type,
                            ..
                        },
                        ir::CellType::Constant { .. },
                    ) => {
                        if !self.shareable.contains(dst_cell_prim_type)
                            && dst.attributes.has("go")
                        {
                            self.go_constants.insert(
                                dst_cell.upgrade().borrow().clone_name(),
                            );
                        }
                    }
                }
            }

            // Something is written to a group: to be added to the graph, this needs to be a "done" port.
            (ir::PortParent::Cell(src_wref), ir::PortParent::Group(_)) => {
                if dst.name == "done" {
                    match src_wref.upgrade().borrow().prototype {
                        ir::CellType::Primitive {
                            name: cell_type, ..
                        }
                        | ir::CellType::Component { name: cell_type } => {
                            if !self.shareable.contains(cell_type) {
                                self.last = Some(
                                    src_wref.upgrade().borrow().clone_name(),
                                );
                            }
                        }
                    }
                }
            }
            // If we encounter anything else, no need to add it to the graph.
            _ => (),
        }
    }
    fn get_cell_names(&self, asmts: &Vec<ir::Assignment>) -> Vec<ir::Id> {
        ReadWriteSet::uses(asmts)
            .filter(|cell| !self.shareable.is_shareable_component(cell))
            .map(|cell| cell.borrow().clone_name())
            .collect()
    }
    fn is_splittable(&mut self, asmts: &Vec<ir::Assignment>) -> bool {
        if !self.build_maps(asmts) {
            return false;
        }
        let list = self.get_cell_names(asmts);
    }
}

impl Visitor for GroupToSeq {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> ir::traversal::VisResult {
        let asmts = s.group.borrow().assignments;
        for asmt in asmts {
            let src = asmt.src;
            let dst = asmt.dst;
            self.update(&src, &dst);
        }
    }
}
