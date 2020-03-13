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

    #[allow(unused)]
    pub fn add_instance(
        &mut self,
        id: &ast::Id,
        comp: &Component,
    ) -> NodeIndex {
        let structure = ast::Structure::decl(id.clone(), id.clone());
        self.structure.add_instance(id, comp, structure)
    }

    pub fn add_primitive(
        &mut self,
        id: &ast::Id,
        name: &str,
        comp: &Component,
        params: &[u64],
    ) -> NodeIndex {
        let structure = ast::Structure::std(
            id.clone(),
            ast::Compinst {
                name: name.to_string(),
                params: params.to_vec(),
            },
        );
        self.structure.add_instance(id, comp, structure)
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

    pub fn get_inst_index(
        &self,
        port: &ast::Id,
    ) -> Result<NodeIndex, errors::Error> {
        self.structure.get_inst_index(port)
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

type CompStore = HashMap<ast::Id, ast::Structure>;

#[allow(unused)]
impl ast::ComponentDef {
    //==========================================
    //        Structure Helper Functions
    //==========================================

    fn from_structure<F, T>(&self, to_data: F) -> Vec<&T>
    where
        F: FnMut(&ast::Structure) -> Option<&T>,
    {
        self.structure.iter().filter_map(to_data).collect()
    }

    /// Get all the wires in this Component.
    pub fn get_wires(&self) -> Vec<&ast::Wire> {
        self.from_structure(|s| match s {
            ast::Structure::Wire { data } => Some(data),
            _ => None,
        })
    }

    /// Get all `Std` from this Component.
    pub fn get_std(&self) -> Vec<&ast::Std> {
        self.from_structure(|s| match s {
            ast::Structure::Std { data } => Some(data),
            _ => None,
        })
    }

    /// Get all `Decl` from this Component.
    pub fn get_decl(&self) -> Vec<&ast::Decl> {
        self.from_structure(|s| match s {
            ast::Structure::Decl { data } => Some(data),
            _ => None,
        })
    }

    /// Build a `CompStore` that contains a mapping from component and Decl
    /// names to clones of the componenets.
    pub fn get_store(&self) -> CompStore {
        self.structure
            .iter()
            .filter_map(|inst| match inst {
                ast::Structure::Std { data } => {
                    Some((data.name.clone(), inst.clone()))
                }
                ast::Structure::Decl { data } => {
                    Some((data.name.clone(), inst.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// Does this Component have an input port with name `port`.
    pub fn has_input_port(&self, port: String) -> bool {
        self.signature
            .inputs
            .iter()
            .any(|in_port| in_port.name == port)
    }

    /// Does this Component have an output port with name `port`.
    pub fn has_output_port(&self, port: String) -> bool {
        self.signature
            .outputs
            .iter()
            .any(|out_port| out_port.name == port)
    }

    /// Get the width for `port`.
    pub fn get_port_width(&self, port: &str) -> u64 {
        self.signature
            .inputs
            .iter()
            .chain(self.signature.outputs.iter())
            .find(|this_port| this_port.name == *port)
            .map(|this_port| this_port.width)
            .expect(
                format!(
                    "Non-existent port: Port {}, Component {}",
                    port, self.name
                )
                .as_str(),
            )
    }
}
