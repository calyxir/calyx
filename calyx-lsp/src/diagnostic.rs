use std::path::Path;

use calyx_opt::{
    passes::{Papercut, SynthesisPapercut, WellFormed},
    traversal::{ConstructVisitor, DiagnosticPass, Visitor},
};
use resolve_path::PathResolveExt;

pub struct Diagnostic;

/// A Calyx error message
#[derive(Debug)]
pub struct CalyxError {
    #[allow(unused)]
    pub file_name: String,
    pub pos_start: usize,
    pub pos_end: usize,
    pub msg: String,
}

impl Diagnostic {
    /// Run the `calyx` compiler on `path` with libraries at `lib_path`
    pub fn did_save(path: &Path, lib_path: &Path) -> Vec<CalyxError> {
        calyx_frontend::Workspace::construct_shallow(
            &Some(path.to_path_buf()),
            lib_path.resolve().as_ref(),
        )
        .and_then(calyx_ir::from_ast::ast_to_ir)
        .and_then(|mut ctx| {
            let mut wellformed = <WellFormed as ConstructVisitor>::from(&ctx)?;
            wellformed.do_pass(&mut ctx)?;

            let mut diag_papercut = <Papercut as ConstructVisitor>::from(&ctx)?;
            diag_papercut.do_pass(&mut ctx)?;

            let mut synth_papercut =
                <SynthesisPapercut as ConstructVisitor>::from(&ctx)?;
            synth_papercut.do_pass(&mut ctx)?;

            Ok(wellformed
                .diagnostics()
                .errors_iter()
                .chain(diag_papercut.diagnostics().errors_iter())
                .chain(synth_papercut.diagnostics().errors_iter())
                .map(|e| {
                    let (file_name, pos_start, pos_end) = e.location();
                    let msg = e.message();
                    CalyxError {
                        file_name: file_name.to_string(),
                        pos_start,
                        pos_end,
                        msg,
                    }
                })
                .collect::<Vec<_>>())
        })
        .unwrap_or_else(|e| {
            let (file_name, pos_start, pos_end) = e.location();
            let msg = e.message();
            vec![CalyxError {
                file_name: file_name.to_string(),
                pos_start,
                pos_end,
                msg,
            }]
        })
    }
}
