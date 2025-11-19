use core::borrow;
use std::collections::{HashMap, HashSet};

use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{
    self as ir, GetAttributes, Id, LibrarySignatures, Primitive, RRC, Rewriter,
    rewriter::{self, RewriteMap},
};
use calyx_utils::CalyxResult;
use itertools::Itertools;

/// Transforms all `par` into `seq`. When the `correctness-checking` option is on,
/// uses [analysis::ControlOrder] to get a sequentialization of `par` such that
/// the program still computes the same value, and errors out when
/// there is no such sequentialization.
///
/// # Example
/// ```
/// par {
///     par { A; B }
///     C;
/// }
/// ```
/// into
/// ```
/// seq { seq { A; B } C; }
/// ```
///
/// To remove uneccessarily nested `par` blocks, run collapse-control.

/// Transforms `if`s with `comb` groups where the `then` block of the `if` consists
/// of a single enable into `if`s with the condition being computed via continuous
/// assignments. (Probably should check if any cells being used in the cond group
/// are used anywhere else)
///
/// # Example
/// ```
/// comb cond_comb {
///     lt.left = x.out;
///     lt.right = 32'd10;
/// }
/// ...
/// if lt.out with cond_comb {
///     then_group;
/// }
/// ```
/// into
///
/// ```
/// // continuous assignments
/// lt.left = x.out;
/// lt.right = 32'd10;
/// ...
/// if lt.out {
///     then_group;
/// }
/// ```
pub struct SimplifyIfComb {}

impl Named for SimplifyIfComb {
    fn name() -> &'static str {
        "simplify-if-comb"
    }

    fn description() -> &'static str {
        "Transform `if` with comb groups into `if` with continuous assignments when there is only one enable in the `then` block and there is no `else` block."
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        // vec![PassOpt::new(
        //     "correctness-checking",
        //     "Errors out when dataflow dependencies cannot be preserved.",
        //     ParseVal::Bool(false),
        //     PassOpt::parse_bool,
        // )]
        vec![]
    }
}

impl ConstructVisitor for SimplifyIfComb {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        // let opts = Self::get_opts(ctx);

        Ok(SimplifyIfComb {
            // correctness_checking: opts[&"correctness-checking"].bool(),
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

// fn create_new_cell(builder: &mut ir::Builder, cell_ref: &RRC<ir::Cell>) {
//     let cell = cell_ref.borrow();
//     match &cell.prototype {
//         calyx_ir::CellType::Primitive {
//             name,
//             param_binding,
//             is_comb,
//             latency,
//         } => todo!(),
//         calyx_ir::CellType::Component { name } => todo!(),
//         calyx_ir::CellType::ThisComponent => todo!(),
//         calyx_ir::CellType::Constant { val, width } => todo!(),
//     }
// }

impl Visitor for SimplifyIfComb {
    // fn start(
    //     &mut self,
    //     comp: &mut calyx_ir::Component,
    //     _sigs: &LibrarySignatures,
    //     _comps: &[calyx_ir::Component],
    // ) -> VisResult {
    //     // for each cell used in a comb_group, track if it's used anywhere else
    //     let mut comb_group_cell_map: HashMap<ir::Id, Vec<ir::Id>> =
    //         HashMap::new();
    //     for comb_group_ref in comp.comb_groups.iter() {
    //         let comb_group = comb_group_ref.borrow();
    //         let comb_group_name = comb_group.name();
    //         let mut cells_this_comb_group = HashSet::new();
    //         // collect cells that are used in this comb group
    //         for comb_group_asgn in &comb_group.assignments {
    //             if let calyx_ir::PortParent::Cell(c) =
    //                 &comb_group_asgn.dst.borrow().parent
    //             {
    //                 let cell_ref = c.upgrade();
    //                 let cell_name = cell_ref.borrow().name();
    //                 cells_this_comb_group.insert(cell_name);
    //             }
    //         }
    //         for cell in cells_this_comb_group {
    //             match comb_group_cell_map.get_mut(&cell) {
    //                 Some(v) => {
    //                     v.push(comb_group_name);
    //                 }
    //                 None => {
    //                     let mut v = Vec::new();
    //                     v.push(comb_group_name);
    //                     comb_group_cell_map.insert(cell, v);
    //                 }
    //             }
    //         }
    //     }
    //     // filter out any cells that were used in two comb groups
    //     comb_group_cell_map.retain(|_, v| v.len() == 1);
    //     // go through normal groups; remove any cells that were used in a normal group
    //     for group_ref in comp.groups.iter() {
    //         let group = group_ref.borrow();
    //         for group_asgn in &group.assignments {
    //             if let calyx_ir::PortParent::Cell(c) =
    //                 &group_asgn.dst.borrow().parent
    //             {
    //                 let cell_ref = c.upgrade();
    //                 let cell_name = cell_ref.borrow().name();
    //                 if comb_group_cell_map.contains_key(&cell_name) {
    //                     comb_group_cell_map.remove(&cell_name);
    //                 }
    //             }
    //         }
    //     }
    //     Ok(Action::Continue)
    // }

    fn finish_if(
        &mut self,
        s: &mut calyx_ir::If,
        comp: &mut calyx_ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let mut rewrite_map = RewriteMap::new();
        match s.tbranch.as_ref() {
            calyx_ir::Control::Enable(enable) => {
                if let Some(cond_group_ref) = &s.cond
                    && s.fbranch.is_empty()
                {
                    // move all assignments in cond group to continuous
                    for cond_group_asgn in &cond_group_ref.borrow().assignments
                    {
                        if let calyx_ir::PortParent::Cell(c) =
                            &cond_group_asgn.dst.borrow().parent
                        {
                            let c_ref = c.upgrade();
                            let c_name = c_ref.borrow().name();
                            // let new_c_name = comp.generate_name(c_name);

                            if let ir::CellType::Primitive {
                                name,
                                param_binding,
                                ..
                            } = &c_ref.borrow().prototype
                            {
                                let new_cell = builder.add_primitive(
                                    c_name,
                                    name.clone(),
                                    &param_binding
                                        .iter()
                                        .map(|(_, v)| *v)
                                        .collect_vec(),
                                );
                                rewrite_map.insert(c_name, new_cell);
                            }
                            // create copy of cell?
                        }
                    }
                    let rewrite = Rewriter {
                        cell_map: rewrite_map,
                        ..Default::default()
                    };
                    for cond_group_asgn in
                        &cond_group_ref.borrow_mut().assignments
                    {
                        let mut new_asgn = cond_group_asgn.clone();
                        rewrite.rewrite_assign(&mut new_asgn);
                        comp.continuous_assignments.push(new_asgn);
                    }
                    // create new enable
                    let new_tbranch =
                        calyx_ir::Control::enable(enable.group.clone());
                    let mut new_if = calyx_ir::Control::if_(
                        s.port.clone(),
                        None,
                        Box::new(new_tbranch),
                        Box::new(calyx_ir::Control::empty()),
                    );
                    rewrite.rewrite_control(&mut new_if);
                    Ok(Action::change(new_if))
                } else {
                    Ok(Action::Continue)
                }
            }
            _ => Ok(Action::Continue),
        }

        // only transform if the true branch consists of a single enable, the false branch is empty, and there is a cond group
        // if let calyx_ir::Control::Enable(_) = *s.tbranch
        //     && let Some(cond_group_ref) = &s.cond
        //     && s.fbranch.is_empty()
        // // there technically needs to be another check here for whether the cells used in the cond group are used in any other group
        // {
        //     // create a new version of the if
        //     let new_if_group = calyx_ir::Control::if_(
        //         s.port.clone(),
        //         None,
        //         Box::new(*(s.tbranch.as_ref())),
        //         s.fbranch,
        //     );
        //     Ok(Action::Continue)
        // } else {
        //     Ok(Action::Continue)
        // }
    }
}
