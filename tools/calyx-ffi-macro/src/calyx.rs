use std::{env, path::PathBuf, rc::Rc};

use proc_macro::TokenStream;

use crate::{parse::CalyxFFIMacroArgs, util};

pub struct CalyxComponent {
    ctx: Rc<calyx_ir::Context>,
    index: usize,
}

impl CalyxComponent {
    pub fn get(&self) -> &calyx_ir::Component {
        &self.ctx.components[self.index]
    }
}

pub fn parse_calyx_file(
    args: &CalyxFFIMacroArgs,
    file: PathBuf,
) -> Result<CalyxComponent, TokenStream> {
    // there has to be a better way to find lib
    let home_dir = env::var("HOME").expect("user home not set");
    let mut lib_path = PathBuf::from(home_dir);
    lib_path.push(".calyx");
    let ws =
        calyx_frontend::Workspace::construct(&Some(file.clone()), &lib_path)
            .map_err(|err| {
                util::compile_error(&args.src_attr_span, err.message())
            })?;
    let ctx = calyx_ir::from_ast::ast_to_ir(ws).map_err(|err| {
        util::compile_error(&args.src_attr_span, err.message())
    })?;

    let comp_index = ctx
        .components
        .iter()
        .position(|comp| comp.name == args.comp)
        .ok_or(util::compile_error(
            &args.comp_attr_span,
            format!(
                "component '{}' does not exist in '{}'",
                args.comp,
                args.src.to_string_lossy()
            ),
        ))?;
    Ok(CalyxComponent {
        ctx: Rc::new(ctx),
        index: comp_index,
    })
}
