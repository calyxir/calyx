use crate::analysis;
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;

/// Transforms all `par` into `seq`. Uses [analysis::ControlOrder] to get a
/// sequentialization of `par` such that the program still computes the same
/// value. When there is no such sequentialization, errors out unless the
/// `always_sequentialize` option is true.
///
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
pub struct ParToSeq {
    /// sequentializes without checking
    /// NOTE: does not guarantee correctness on shared reads and writes
    always_sequentialize: bool,
}

impl Named for ParToSeq {
    fn name() -> &'static str {
        "par-to-seq"
    }

    fn description() -> &'static str {
        "Transform `par` blocks to `seq`"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![PassOpt::new(
            "always-sequentialize",
            "Sequentializes the program without attempting to preserve dataflow dependencies.",
            ParseVal::Bool(false),
            PassOpt::parse_bool,
        )]
    }
}

impl ConstructVisitor for ParToSeq {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        Ok(ParToSeq {
            always_sequentialize: opts[&"always-sequentialize"].bool(),
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Visitor for ParToSeq {
    /// Collapse par { par { A }; B } into par { A; B }.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _c: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let par = if self.always_sequentialize {
            ir::Control::seq(s.stmts.drain(..).collect())
        } else {
            let total_order = analysis::ControlOrder::<true>::get_total_order(
                s.stmts.drain(..),
            )?;
            ir::Control::seq(total_order)
        };
        Ok(Action::change(par))
    }
}
