use super::visitor::{Changes, Visitor};
use crate::lang::ast::{ComponentDef, Port, Portdef, Structure, Wire};

pub struct ReadWire {}

impl ReadWire {
    pub fn new() -> Self {
        ReadWire {}
    }
}

fn update_port(port: &Port, port_suffix: &str) -> Port {
    match port {
        Port::Comp { component, port } => Port::Comp {
            component: component.to_string(),
            port: format!("{}_{}", port, port_suffix),
        },
        Port::This { port } => Port::This {
            port: format!("{}_{}", port, port_suffix),
        },
    }
}

impl Visitor<()> for ReadWire {
    fn name(&self) -> String {
        "Duplicate every data wire to implement Futil semantics".to_string()
    }

    fn start(
        &mut self,
        comp: &mut ComponentDef,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        for port in &comp.signature.inputs {
            let read_for_port = Portdef {
                name: format!("{}_read_in", port.name),
                width: 1,
            };
            changes.add_input_port(read_for_port)
        }

        for port in &comp.signature.outputs {
            let read_for_port = Portdef {
                name: format!("{}_read_out", port.name),
                width: 1,
            };
            changes.add_input_port(read_for_port)
        }

        for wire in &comp.structure {
            match wire {
                Structure::Wire { data } => {
                    let read_wire = Wire {
                        src: update_port(&data.src, "read_out"),
                        dest: update_port(&data.dest, "read_in"),
                    };
                    println!("{:?}", read_wire);
                    changes.add_structure(Structure::Wire { data: read_wire })
                }
                _ => (),
            }
        }

        // return err to avoid touching every control node
        changes.commit();
        Err(())
    }
}
