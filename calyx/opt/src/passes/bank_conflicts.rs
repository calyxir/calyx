use crate::analysis::BankConflictAnalysis;
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir as ir;
use calyx_utils::{CalyxResult, OutputFile};
use std::io::Write;

/// Prints the result of [`BankConflictAnalysis`] for every component.
///
/// The pass does not modify the program. For each component it writes a
/// `component <name>` line followed by one line per conflict, minimum bank
/// count, bank assignment and loop-carried dependence.
pub struct BankConflicts {
    report: Option<OutputFile>,
}

impl Named for BankConflicts {
    fn name() -> &'static str {
        "bank-conflicts"
    }

    fn description() -> &'static str {
        "Report memory accesses that share a cycle, the banks needed to separate them and loop-carried dependences"
    }

    fn opts() -> Vec<PassOpt> {
        vec![PassOpt::new(
            "report",
            "Where to write the report",
            ParseVal::OutStream(OutputFile::Stderr),
            PassOpt::parse_outstream,
        )]
    }
}

impl ConstructVisitor for BankConflicts {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);
        Ok(BankConflicts {
            report: opts[&"report"].not_null_outstream(),
        })
    }

    fn clear_data(&mut self) {}
}

impl Visitor for BankConflicts {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let Some(stream) = &mut self.report else {
            return Ok(Action::Stop);
        };
        let report = BankConflictAnalysis::analyze(comp);
        let mut out = stream.get_write();
        writeln!(out, "component {}", comp.name).unwrap();
        write!(out, "{report}").unwrap();
        Ok(Action::Stop)
    }
}
