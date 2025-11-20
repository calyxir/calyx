use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures, Rewriter, rewriter::RewriteMap};
use calyx_utils::CalyxResult;
use itertools::Itertools;

/// Transforms `if`s with `comb` groups where the `then` block of the `if` consists
/// of a single enable into `if`s with the condition being computed via continuous
/// assignments.
///
/// The cell used for the condition (and any other cells on the LHS of an assignment
/// in the comb group) will be cloned. Therefore, it is important that dead-cell-removal
/// and dead-group-removal is ran after this pass.
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
/// lt0.left = x.out;
/// lt0.right = 32'd10;
/// ...
/// if lt0.out {
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
        vec![]
    }
}

impl ConstructVisitor for SimplifyIfComb {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        Ok(SimplifyIfComb {})
    }

    fn clear_data(&mut self) {}
}

impl Visitor for SimplifyIfComb {
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

                            if !rewrite_map.contains_key(&c_name)
                                && let ir::CellType::Primitive {
                                    name,
                                    param_binding,
                                    ..
                                } = &c_ref.borrow().prototype
                            {
                                let new_cell = builder.add_primitive(
                                    c_name,
                                    *name,
                                    &param_binding
                                        .iter()
                                        .map(|(_, v)| *v)
                                        .collect_vec(),
                                );
                                rewrite_map.insert(c_name, new_cell);
                            }
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
                    // create new enable for the true branch
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
    }
}
