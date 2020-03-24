use calyx::lang::{ast, context, library::ast as lib};
use calyx::{backend::traits::Backend, backend::verilog::gen::VerilogBackend};
use calyx::{errors, passes, passes::visitor::Visitor};
use sexpy::Sexpy;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

fn compile(library: &str, namespace: &str) -> Result<String, errors::Error> {
    let namespace_ast = ast::NamespaceDef::parse(namespace)?;
    let lib_ast = lib::Library::parse(library)?;

    let context = context::Context::from_ast(namespace_ast, &[lib_ast])?;

    passes::lat_insensitive::LatencyInsenstive::do_pass_default(&context)?;
    passes::redundant_par::RedundantPar::do_pass_default(&context)?;
    passes::remove_if::RemoveIf::do_pass_default(&context)?;
    passes::collapse_seq::CollapseSeq::do_pass_default(&context)?;

    let mut buffer: Vec<u8> = vec![];
    VerilogBackend::run(&context, &mut buffer)?;
    Ok(String::from_utf8(buffer).unwrap())
}

#[wasm_bindgen]
pub fn run(library: &str, namespace: &str) -> String {
    match compile(library, namespace) {
        Ok(s) => s,
        Err(e) => format!("{:?}", e),
    }
}
