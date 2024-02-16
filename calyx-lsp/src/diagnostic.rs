use std::path::PathBuf;

use calyx_frontend;
use calyx_ir;

pub struct Diagnostic;

#[derive(Debug)]
pub struct CalyxError {
    #[allow(unused)]
    pub file_name: String,
    pub pos_start: usize,
    pub pos_end: usize,
    pub msg: String,
}

impl Diagnostic {
    pub fn did_save(path: &PathBuf, lib_path: &PathBuf) -> Vec<CalyxError> {
        calyx_frontend::Workspace::construct(&Some(path.clone()), lib_path)
            .and_then(|ws| calyx_ir::from_ast::ast_to_ir(ws))
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
