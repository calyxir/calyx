use crate::lang::ast;
use crate::lang::library;
use petgraph::graph::{Graph, NodeIndex};
use std::collections::HashMap;

impl library::ast::Primitive {
    pub fn get_port_width(&self, inst: ast::Structure, port: &String) -> i64 {
        match inst {
            ast::Structure::Std { data } => {
                let params = data.instance.params;

                for in_port in &self.inputs {
                    if in_port.name == *port {
                        match &in_port.width {
                            library::ast::Width::Const { value } => {
                                return value.clone()
                            }
                            library::ast::Width::Param { value } => {
                                for i in 0..params.len() {
                                    if self.params[i] == *value {
                                        return params[i];
                                    }
                                }
                                panic!("Parameter For Input Port Not Found: Port {}, Param {}", port, value);
                            }
                        }
                    }
                }
                for out_port in &self.outputs {
                    if out_port.name == *port {
                        match &out_port.width {
                            library::ast::Width::Const { value } => {
                                return value.clone()
                            }
                            library::ast::Width::Param { value } => {
                                for i in 0..params.len() {
                                    if self.params[i] == *value {
                                        return params[i];
                                    }
                                }
                                panic!("Parameter For Output Port Not Found: Port {}, Param {}", port, value);
                            }
                        }
                    }
                }
                panic!(
                    "Non-existent port: Port {}, Component {}",
                    port, self.name
                )
            }
            _ => panic!("Expected Std declaration in get_port_width primitive"),
        }
    }
}
