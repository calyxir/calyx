use crate::cmdline::Opts;
use crate::errors;
use crate::lang::ast;
use crate::lang::structure;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Context {
    definitions: HashMap<ast::Id, (ast::Control, structure::StructureGraph)>,
}

impl Context {
    pub fn from_opts(opts: &Opts) -> Result<Self, errors::Error> {
        // parse file
        let namespace = ast::parse_file(&opts.file)?;

        // build hashmap from components in `namespace`
        let mut definitions = HashMap::new();
        for comp in namespace.components {
            let graph = comp.structure_graph()?;
            definitions.insert(comp.name.clone(), (comp.control, graph));
        }

        Ok(Context { definitions })
    }
}
