use std::path::Path;
use tower_lsp::lsp_types::{self as lspt};

use calyx_opt::{
    passes::{Papercut, SynthesisPapercut, WellFormed},
    traversal::{ConstructVisitor, DiagnosticPass, Visitor},
};
use resolve_path::PathResolveExt;

use crate::document::Document;

pub struct Diagnostic;

/// A Calyx error message
#[derive(Debug)]
pub struct CalyxError {
    #[allow(unused)]
    pub file_name: String,
    pub pos_start: usize,
    pub pos_end: usize,
    pub msg: String,
    pub annotations: Vec<(String, usize, usize)>,
    severity: lspt::DiagnosticSeverity,
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

            let warnings = wellformed
                .diagnostics()
                .warning_iter()
                .chain(diag_papercut.diagnostics().warning_iter())
                .chain(synth_papercut.diagnostics().warning_iter())
                .map(|e| {
                    let (file_name, pos_start, pos_end) = e.location();
                    let msg = e.message();
                    let annotations = e.annotations();
                    CalyxError {
                        file_name: file_name.to_string(),
                        pos_start,
                        pos_end,
                        msg,
                        annotations,
                        severity: lspt::DiagnosticSeverity::WARNING,
                    }
                });

            Ok(wellformed
                .diagnostics()
                .errors_iter()
                .chain(diag_papercut.diagnostics().errors_iter())
                .chain(synth_papercut.diagnostics().errors_iter())
                .map(|e| {
                    let (file_name, pos_start, pos_end) = e.location();
                    let msg = e.message();
                    let annotations = e.annotations();
                    CalyxError {
                        file_name: file_name.to_string(),
                        pos_start,
                        pos_end,
                        msg,
                        annotations,
                        severity: lspt::DiagnosticSeverity::ERROR,
                    }
                })
                .chain(warnings)
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
                annotations: vec![],
                severity: lspt::DiagnosticSeverity::ERROR,
            }]
        })
    }
}

impl CalyxError {
    pub fn into_lspt_diagnostics(
        self,
        doc: &Document,
    ) -> Vec<lspt::Diagnostic> {
        // convert annotations into related information
        // this however doesn't actually highlight the locations.
        // instead is just shows up in the error tooltop in VSCode.
        let related_information = self
            .annotations
            .iter()
            .filter_map(|(msg, start, end)| {
                doc.bytes_to_range(*start, *end).map(|range| {
                    lspt::DiagnosticRelatedInformation {
                        location: lspt::Location::new(
                            doc.url.clone(),
                            range.into(),
                        ),
                        message: msg.to_string(),
                    }
                })
            })
            .collect();

        // also translate annotations into diagnostics
        let annotation_diagnostics =
            self.annotations.iter().filter_map(|(msg, start, end)| {
                doc.bytes_to_range(*start, *end)
                    .map(|range| lspt::Diagnostic {
                        range: range.into(),
                        severity: Some(lspt::DiagnosticSeverity::INFORMATION),
                        code: None,
                        code_description: None,
                        source: Some("calyx-lsp".to_string()),
                        message: msg.to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    })
            });

        doc.bytes_to_range(self.pos_start, self.pos_end)
            .map(|range| lspt::Diagnostic {
                range: range.into(),
                severity: Some(self.severity),
                code: None,
                code_description: None,
                source: Some("calyx-lsp".to_string()),
                message: self.msg,
                related_information: Some(related_information),
                tags: None,
                data: None,
            })
            .into_iter()
            .chain(annotation_diagnostics)
            .collect()
    }
}
