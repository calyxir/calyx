use std::path::Path;

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
        calyx_frontend::Workspace::construct(
            &Some(path.to_path_buf()),
            lib_path.resolve().as_ref(),
        )
        .and_then(calyx_ir::from_ast::ast_to_ir)
        // TODO: call well-formed pass
        .map(|_| vec![])
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
