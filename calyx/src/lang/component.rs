use crate::lang::ast;
use petgraph::graph::{Graph, NodeIndex};
use std::collections::HashMap;

type CompStore = HashMap<ast::Id, ast::Structure>;

impl ast::Component {
    //==========================================
    //        Structure Helper Functions
    //==========================================

    fn get_wires(&self) -> Vec<ast::Wire> {
        let mut v: Vec<ast::Wire> = Vec::new();
        for structure in self.structure.iter() {
            match structure {
                ast::Structure::Wire { data } => v.push(data.clone()),
                _ => {}
            }
        }
        v
    }

    fn get_std(&self) -> Vec<ast::Std> {
        let mut v: Vec<ast::Std> = Vec::new();
        for structure in self.structure.iter() {
            match structure {
                ast::Structure::Std { data } => v.push(data.clone()),
                _ => {}
            }
        }
        v
    }

    fn get_new(&self) -> Vec<ast::Decl> {
        let mut v: Vec<ast::Decl> = Vec::new();
        for structure in self.structure.iter() {
            match structure {
                ast::Structure::Decl { data } => v.push(data.clone()),
                _ => {}
            }
        }
        v
    }
    fn get_store(&self) -> CompStore {
        let mut store: CompStore = HashMap::new();
        let std = self.get_std();
        let new = self.get_new();
        for inst in std {
            store.insert(inst.name.clone(), ast::Structure::Std { data: inst });
        }
        for inst in new {
            store
                .insert(inst.name.clone(), ast::Structure::Decl { data: inst });
        }
        store
    }
}
