use std::collections::HashMap;

use calyx::{
    errors::{self, FutilResult},
    frontend::parser,
    ir::{self, traversal::Named, traversal::Visitor},
    passes::*,
};
use wasm_bindgen::prelude::*;

type PassClosure = Box<dyn Fn(&mut ir::Context) -> FutilResult<()>>;

macro_rules! register {
    ($map:expr, $pass:ident) => {
        $map.insert(
            $pass::name().to_string(),
            Box::new(|ir| {
                $pass::do_pass_default(ir)?;
                Ok(())
            }),
        );
    };
}

fn compile(
    passes: &[String],
    library: &str,
    namespace: &str,
) -> Result<String, errors::Error> {
    let mut pm: HashMap<String, PassClosure> = HashMap::new();
    register!(pm, WellFormed);
    register!(pm, Papercut);
    register!(pm, Externalize);
    register!(pm, CompileInvoke);
    register!(pm, CollapseControl);
    register!(pm, CompileControl);
    register!(pm, InferStaticTiming);
    register!(pm, ResourceSharing);
    register!(pm, MinimizeRegs);
    register!(pm, CompileEmpty);
    register!(pm, StaticTiming);
    register!(pm, CompileControl);
    register!(pm, DeadCellRemoval);
    register!(pm, GoInsertion);
    register!(pm, ComponentInterface);
    register!(pm, Inliner);
    register!(pm, ClkInsertion);

    let namespace_ast = parser::FutilParser::parse(
        (library.to_string() + "\n" + namespace).as_bytes(),
    )?;

    // Build the IR representation
    let mut rep = ir::from_ast::ast_to_ir(namespace_ast, false, false)?;

    for name in passes {
        pm[name](&mut rep)?;
    }

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
