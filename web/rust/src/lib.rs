use calyx::{
    errors::{self, CalyxResult},
    frontend, ir,
    pass_manager::PassManager,
};
use wasm_bindgen::prelude::*;

// Contruct a workspace from a namspace
fn ws_from_ns(ns: frontend::NamespaceDef) -> CalyxResult<frontend::Workspace> {
    if !ns.imports.is_empty() {
        return Err(errors::Error::Misc(
            "import not supported in the web demo".to_string(),
        ));
    }
    let mut ws = frontend::Workspace::default();
    ws.externs
        .extend(ns.externs.into_iter().map(|(p, es)| (p.into(), es)));

    // Add components defined by this namespace to either components or
    // declarations
    ws.components.extend(&mut ns.components.into_iter());

    Ok(ws)
}

fn compile(
    passes: &[String],
    library: &str,
    namespace: &str,
) -> Result<String, errors::Error> {
    let pm = PassManager::default_passes()?;

    let ns = frontend::parser::CalyxParser::parse(
        (library.to_string() + "\n" + namespace).as_bytes(),
    )?;
    let ws = ws_from_ns(ns)?;

    // Build the IR representation
    let bc = ir::BackendConf::default();
    let mut rep = ir::from_ast::ast_to_ir(ws, bc)?;

    pm.execute_plan(&mut rep, passes, &[])?;

    let mut buffer: Vec<u8> = vec![];
    for comp in &rep.components {
        ir::IRPrinter::write_component(comp, &mut buffer)?;
    }
    Ok(String::from_utf8(buffer).unwrap())
}

#[wasm_bindgen]
pub fn run(passes: &JsValue, library: &str, namespace: &str) -> String {
    let test: Vec<String> = passes.into_serde().unwrap();
    match compile(&test, library, namespace) {
        Ok(s) => s,
        Err(e) => format!("Error:\n{:?}", e),
    }
}
