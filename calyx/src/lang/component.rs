use super::{ast, structure::StructureGraph};
use crate::errors;
use crate::lang::pretty_print::PrettyPrint;
use petgraph::graph::NodeIndex;
use pretty::{termcolor::ColorSpec, RcDoc};
use std::collections::HashMap;

/// In memory representation for a Component. Contains a Signature, Control AST,
/// structure graph, and resolved signatures of used components
#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub signature: ast::Signature,
    pub control: ast::Control,
    pub structure: StructureGraph,
    pub resolved_sigs: HashMap<ast::Id, ast::Signature>,
}

impl Component {
    pub fn from_signature(name: &str, sig: ast::Signature) -> Self {
        let mut graph = StructureGraph::new();
        graph.add_signature(&sig);

        Component {
            name: name.to_string(),
            signature: sig,
            control: ast::Control::empty(),
            structure: graph,
            resolved_sigs: HashMap::new(),
        }
    }

    pub fn add_input(&mut self, portdef: impl Into<ast::Portdef>) {
        let portdef = portdef.into();
        self.structure.insert_input_port(&portdef);
        self.signature.inputs.push(portdef);
    }

    pub fn add_output(&mut self, portdef: impl Into<ast::Portdef>) {
        let portdef = portdef.into();
        self.structure.insert_output_port(&portdef);
        self.signature.outputs.push(portdef);
    }

    pub fn add_instance(
        &mut self,
        id: &ast::Id,
        comp: &Component,
    ) -> NodeIndex {
        self.structure.add_instance(id, comp)
    }

    pub fn add_wire(
        &mut self,
        src_comp: NodeIndex,
        src_port: &str,
        dest_comp: NodeIndex,
        dest_port: &str,
    ) -> Result<(), errors::Error> {
        self.structure
            .insert_edge(src_comp, src_port, dest_comp, dest_port)
    }

    pub fn get_io_index(&self, port: &str) -> Result<NodeIndex, errors::Error> {
        self.structure.get_io_index(port)
    }
}

impl Into<ast::ComponentDef> for Component {
    fn into(self) -> ast::ComponentDef {
        ast::ComponentDef {
            name: self.name,
            signature: self.signature,
            structure: self.structure.into(),
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

// use crate::lang::ast;
// use std::collections::HashMap;

type CompStore = HashMap<ast::Id, ast::Structure>;

#[allow(unused)]
impl ast::ComponentDef {
    //==========================================
    //        Structure Helper Functions
    //==========================================

    pub fn get_wires(&self) -> Vec<&ast::Wire> {
        let mut v: Vec<&ast::Wire> = Vec::new();
        for structure in self.structure.iter() {
            if let ast::Structure::Wire { data } = structure {
                v.push(data)
            }
        }
        v
    }

    pub fn get_std(&self) -> Vec<&ast::Std> {
        let mut v: Vec<&ast::Std> = Vec::new();
        for structure in self.structure.iter() {
            if let ast::Structure::Std { data } = structure {
                v.push(data)
            }
        }
        v
    }

    pub fn get_decl(&self) -> Vec<&ast::Decl> {
        let mut v: Vec<&ast::Decl> = Vec::new();
        for structure in self.structure.iter() {
            if let ast::Structure::Decl { data } = structure {
                v.push(data)
            }
        }
        v
    }

    pub fn get_store(&self) -> CompStore {
        let mut store: CompStore = HashMap::new();
        let std = self.get_std();
        let new = self.get_decl();
        for inst in std {
            store.insert(
                inst.name.clone(),
                ast::Structure::Std { data: inst.clone() },
            );
        }
        for inst in new {
            store.insert(
                inst.name.clone(),
                ast::Structure::Decl { data: inst.clone() },
            );
        }
        store
    }

    pub fn has_input_port(&self, port: String) -> bool {
        for in_port in &self.signature.inputs {
            if in_port.name == port {
                return true;
            }
        }
        false
    }

    pub fn has_output_port(&self, port: String) -> bool {
        for out_port in &self.signature.outputs {
            if out_port.name == port {
                return true;
            }
        }
        false
    }

    pub fn get_port_width(&self, port: &str) -> u64 {
        for in_port in &self.signature.inputs {
            if in_port.name == *port {
                return in_port.width;
            }
        }
        for out_port in &self.signature.outputs {
            if out_port.name == *port {
                return out_port.width;
            }
        }
        panic!("Non-existent port: Port {}, Component {}", port, self.name)
    }
}
