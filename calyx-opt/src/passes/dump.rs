use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, Visitor,
};

use calyx_ir as ir;
use calyx_utils::CalyxResult;

/// Prints the current component
pub struct Dump {
    /// Print to stderr instead of stdout
    stderr: bool,
}

impl Named for Dump {
    fn name() -> &'static str {
        "dump"
    }

    fn description() -> &'static str {
        "Prints the current IR"
    }

    fn opts() -> Vec<PassOpt> {
        vec![PassOpt::new(
            "stderr",
            "Print to stderr instead of stdout",
            ParseVal::Bool(false),
            PassOpt::parse_bool,
        )]
    }
}

impl ConstructVisitor for Dump {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);
        Ok(Dump {
            stderr: opts[&"stderr"].bool(),
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Visitor for Dump {
    fn start(
        &mut self,
        comp: &mut calyx_ir::Component,
        _sigs: &calyx_ir::LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> crate::traversal::VisResult {
        if self.stderr {
            ir::Printer::write_component(&comp, &mut std::io::stderr())?;
            eprintln!();
        } else {
            ir::Printer::write_component(&comp, &mut std::io::stdout())?;
            println!();
        }
        Ok(Action::Continue)
    }
}
