use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;

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

impl Visitor for SimplifyIfComb {
    fn finish_if(
        &mut self,
        s: &mut calyx_ir::If,
        comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        // let mut builder = ir::Builder::new(comp, sigs);
        match s.tbranch.as_ref() {
            calyx_ir::Control::Enable(enable) => {
                if let Some(cond_group_ref) = &s.cond
                    && s.fbranch.is_empty()
                {
                    // move all assignments in cond group to continuous
                    for cond_group_asgn in &cond_group_ref.borrow().assignments
                    {
                        comp.continuous_assignments
                            .push(cond_group_asgn.clone());
                    }
                    // create new enable
                    let new_tbranch =
                        calyx_ir::Control::enable(enable.group.clone());
                    let new_if = calyx_ir::Control::if_(
                        s.port.clone(),
                        None,
                        Box::new(new_tbranch),
                        Box::new(calyx_ir::Control::empty()),
                    );
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
