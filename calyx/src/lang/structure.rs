use crate::lang::ast;
use petgraph::dot::Dot;
use petgraph::graph::{Graph, NodeIndex};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum StructureStmt {
    Decl {
        name: ast::Id,
        component: String,
    },
    Std {
        name: ast::Id,
        instance: ast::Compinst,
    },
    Wire {
        src: ast::Port,
        dest: ast::Port,
    },
}

type StructGraph = Graph<ast::Id, ()>;

// I want to keep the fields of this struct private so that it is easy to swap
// out implementations / add new ways of manipulating this
/** Structure holds information about the structure of the current component. */
#[derive(Clone, Debug)]
pub struct Structure {
    // stmts: Vec<StructureStmt>,
    node_hash: HashMap<ast::Id, NodeIndex>,
    graph: StructGraph,
}

impl ast::Port {
    fn into_id(&self) -> &ast::Id {
        match self {
            ast::Port::Comp { component, port: _ } => component,
            ast::Port::This { port } => port,
        }
    }
}

impl ast::Structure {
    // Control the creation method of Structure
    pub fn new(&self) -> Structure {
        let mut g = StructGraph::new();
        let mut node_hash = HashMap::new();
        // add vertices
        for stmt in self {
            match stmt {
                StructureStmt::Decl { name, component: _ } => {
                    node_hash.insert(name.clone(), g.add_node(name.clone()));
                }
                StructureStmt::Std { name, instance: _ } => {
                    node_hash.insert(name.clone(), g.add_node(name.clone()));
                }
                StructureStmt::Wire { src: _, dest: _ } => (),
            }
        }

        // add edges
        for stmt in &stmts {
            match stmt {
                StructureStmt::Decl {
                    name: _,
                    component: _,
                }
                | StructureStmt::Std {
                    name: _,
                    instance: _,
                } => (),
                StructureStmt::Wire { src, dest } => {
                    match (
                        node_hash.get(src.into_id()),
                        node_hash.get(dest.into_id()),
                    ) {
                        (Some(s), Some(d)) => {
                            g.add_edge(*s, *d, ());
                        }
                        _ => panic!(
                            "Used an undeclared component in a connection: {:?} -> {:?}",
                            src.into_id(), dest.into_id()
                        ),
                    }
                }
            }
        }

        Structure {
            node_hash: node_hash,
            graph: g,
        }
    }

    pub fn visualize(&self) -> () {
        println!("{:?}", Dot::new(&self.graph))
    }
    // more future methods for manipulating the structure
}
