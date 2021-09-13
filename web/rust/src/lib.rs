use calyx::{
    errors,
    frontend::parser,
    ir,
    pass_manager::PassManager,
};
use wasm_bindgen::prelude::*;

fn compile(
    passes: &[String],
    library: &str,
    namespace: &str,
) -> Result<String, errors::Error> {
    let pm = PassManager::default_passes()?;

    let namespace_ast = parser::CalyxParser::parse(
        (library.to_string() + "\n" + namespace).as_bytes(),
    )?;

    // Build the IR representation
    let mut rep = ir::from_ast::ast_to_ir(namespace_ast, false, false)?;

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
