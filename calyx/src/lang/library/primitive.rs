use crate::lang::ast;
use crate::lang::library;

impl library::ast::Primitive {
    #[allow(unused)]
    pub fn get_port_width(&self, inst: ast::Structure, port: &str) -> i64 {
        match inst {
            ast::Structure::Std { data } => {
                let params = data.instance.params;

                for in_port in &self.inputs {
                    if in_port.name == *port {
                        match &in_port.width {
                            library::ast::Width::Const { value } => {
                                return *value
                            }
                            library::ast::Width::Param { value } => {
                                for (i, x) in params.iter().enumerate() {
                                    if self.params[i] == *value {
                                        return *x;
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
                                return *value;
                            }
                            library::ast::Width::Param { value } => {
                                for (i, x) in params.iter().enumerate() {
                                    if self.params[i] == *value {
                                        return *x;
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
