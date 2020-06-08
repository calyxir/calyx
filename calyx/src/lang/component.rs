use super::ast;
use crate::errors::{Error, Result};
use crate::frontend::pretty_print::PrettyPrint;
use crate::lang::structure::StructureGraph;
use pretty::{termcolor::ColorSpec, RcDoc};
use std::collections::HashMap;

/// In memory representation for a Component. Contains a Signature, Control AST,
/// structure graph, and resolved signatures of used components
#[derive(Debug, Clone)]
pub struct Component {
    pub name: ast::Id,
    pub signature: ast::Signature,
    pub control: ast::Control,
    pub structure: StructureGraph,
    /// Maps names of sub-component used in this component to fully
    /// resolved signatures.
    pub resolved_sigs: HashMap<ast::Id, ast::Signature>,
}

/// Methods over Components. Only define functions that cannot be methods
/// on `Control`, `Signature`, or `Structure`.
impl Component {
    pub fn from_signature<S: AsRef<str>>(name: S, sig: ast::Signature) -> Self {
        let mut graph = StructureGraph::default();
        graph.add_signature(&sig);

        Component {
            name: name.as_ref().into(),
            signature: sig,
            control: ast::Control::empty(),
            structure: graph,
            resolved_sigs: HashMap::new(),
        }
    }

    // XXX(rachit): Document this function.
    pub fn add_input(
        &mut self,
        portdef: impl Into<ast::Portdef>,
    ) -> Result<()> {
        let portdef = portdef.into();
        if !self.signature.has_input(portdef.name.as_ref()) {
            // self.structure.insert_input_port(&portdef);
            self.signature.inputs.push(portdef);
            Ok(())
        } else {
            Err(Error::DuplicatePort(self.name.clone(), portdef))
        }
    }

    // XXX(rachit): Document this function.
    pub fn add_output(
        &mut self,
        portdef: impl Into<ast::Portdef>,
    ) -> Result<()> {
        let portdef = portdef.into();
        if !self.signature.has_output(portdef.name.as_ref()) {
            // self.structure.insert_output_port(&portdef);
            self.signature.outputs.push(portdef);
            Ok(())
        } else {
            Err(Error::DuplicatePort(self.name.clone(), portdef))
        }
    }
}

impl Into<ast::ComponentDef> for Component {
    fn into(self) -> ast::ComponentDef {
        let (cells, connections) = self.structure.into();
        ast::ComponentDef {
            name: self.name,
            signature: self.signature,
            cells,
            connections,
            control: self.control,
        }
    }
}

impl PrettyPrint for Component {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let v: ast::ComponentDef = self.clone().into();
        let vref = arena.alloc(v);
        vref.prettify(&arena)
    }
}
