use super::visitor::{Changes, Visitor};
use crate::lang::ast::{Component, Control, Port, Portdef, Structure};
use crate::utils::NameGenerator;
use std::collections::HashMap;

pub struct Lookup<'a> {
    names: &'a mut NameGenerator,
}

impl Lookup<'_> {
    pub fn new(names: &mut NameGenerator) -> Lookup {
        Lookup { names }
    }
}

fn get_port_name(port: &Port) -> &str {
    match port {
        Port::Comp { port, .. } => port,
        Port::This { port } => port,
    }
}

impl Visitor<()> for Lookup<'_> {
    fn name(&self) -> String {
        "Add lookup logic for control wires".to_string()
    }

    fn start(
        &mut self,
        component: &mut Component,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        let mut sources: HashMap<Port, Vec<Port>> = HashMap::new();

        for stmt in &component.structure {
            match stmt {
                Structure::Wire { data } => {
                    let mut srcs =
                        sources.get(&data.dest).map_or(vec![], |x| x.clone());
                    srcs.push(data.src.clone());
                    sources.insert(data.dest.clone(), srcs);
                }
                _ => (),
            }
        }

        for (dest, srcs) in sources {
            if srcs.len() > 1 {
                if get_port_name(&dest).starts_with("valid")
                    || get_port_name(&dest).starts_with("ready")
                {
                    let name = self.names.gen_name("lut_control_");
                    let inputs: Vec<Portdef> = srcs
                        .iter()
                        .map(|_| Portdef {
                            name: self.names.gen_name(&get_port_name(&dest)),
                            width: 1,
                        })
                        .collect();
                    let output = Portdef {
                        name: get_port_name(&dest).to_string(),
                        width: 1,
                    };
                    let component = Component {
                        name: name.clone(),
                        inputs: inputs.clone(),
                        outputs: vec![output.clone()],
                        structure: vec![],
                        control: Control::empty(),
                    };

                    changes.add_structure(Structure::decl(
                        component.name.clone(),
                        component.name.clone(),
                    ));

                    for (src, lut_dest) in srcs.iter().zip(inputs) {
                        let wire = Structure::wire(
                            src.clone(),
                            Port::Comp {
                                component: name.clone(),
                                port: lut_dest.name,
                            },
                        );
                        changes.add_structure(wire);
                    }

                    let output_wire = Structure::wire(
                        Port::Comp {
                            component: name,
                            port: output.name,
                        },
                        dest.clone(),
                    );

                    changes.add_structure(output_wire);
                    changes.add_component(component);

                    let mut structs = srcs
                        .iter()
                        .map(|p| Structure::wire(p.clone(), dest.clone()))
                        .collect();
                    changes.batch_remove_structure(&mut structs);
                }
            }
        }

        Err(())
    }
}
