use crate::analysis;
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
pub struct ParToSeq {
    /// Guarantees correctness on shared reads and writes by erroring out
    /// if sequentialization where program still computes the same value doesn't exist.
    correctness_checking: bool,
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
            "correctness-checking",
            "Errors out when dataflow dependencies cannot be preserved.",
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
            correctness_checking: opts[&"correctness-checking"].bool(),
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
        let par = if self.correctness_checking {
            let total_order = analysis::ControlOrder::<true>::get_total_order(
                s.stmts.drain(..),
            )?;
            ir::Control::seq(total_order)
        } else {
            ir::Control::seq(s.stmts.drain(..).collect())
        };
        Ok(Action::change(par))
    }
}
