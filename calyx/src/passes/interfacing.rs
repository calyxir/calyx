use crate::lang::ast::{ComponentDef, Port, Portdef, Structure, Wire};
use crate::passes::visitor::{Changes, Visitor};

pub struct Interfacing {}

impl Interfacing {
    pub fn new() -> Self {
        Interfacing {}
    }
}

impl Visitor<()> for Interfacing {
    fn name(&self) -> String {
        "Add Interfacing for toplevel component".to_string()
    }

    fn start(
        &mut self,
        component: &mut ComponentDef,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        // add clk port to all components
        let clk = Portdef {
            name: "clk".to_string(),
            width: 1,
        };

        changes.add_input_port(clk);

        // connect component clk to sub-component clks
        for stmt in &component.structure {
            match stmt {
                Structure::Std { data } => {
                    if &data.instance.name == "std_reg" {
                        let clk_wire = Wire {
                            src: Port::This {
                                port: "clk".to_string(),
                            },
                            dest: Port::Comp {
                                component: data.name.clone(),
                                port: "clk".to_string(),
                            },
                        };
                        changes
                            .add_structure(Structure::Wire { data: clk_wire });
                    }
                }
                Structure::Decl { data } => {
                    let clk_wire = Wire {
                        src: Port::This {
                            port: "clk".to_string(),
                        },
                        dest: Port::Comp {
                            component: data.name.clone(),
                            port: "clk".to_string(),
                        },
                    };
                    changes.add_structure(Structure::Wire { data: clk_wire });
                }
                _ => (),
            }
        }

        // return err to avoid recursing down entire control tree
        changes.commit();
        Err(())
    }
}
