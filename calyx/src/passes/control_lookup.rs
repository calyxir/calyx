use super::visitor::{Changes, Visitor};
use crate::lang::ast::{ComponentDef, Control, Port, Portdef, Structure};
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
fn get_comp_name(port: &Port) -> &str {
    match port {
        Port::Comp { component, .. } => component,
        Port::This { .. } => panic!("necessary to have component name"),
    }
}

impl Visitor<()> for Lookup<'_> {
    fn name(&self) -> String {
        "Add lookup logic for control wires".to_string()
    }

    fn start(
        &mut self,
        component: &mut ComponentDef,
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

        for (dest, srcs) in sources.clone() {
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
                    let component = ComponentDef {
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
                        println!("{:?}", wire);
                        changes.add_structure(wire);
                    }

                    let output_wire = Structure::wire(
                        Port::Comp {
                            component: name,
                            port: output.name,
                        },
                        dest.clone(),
                    );

                    println!("{:?}\n", output_wire);
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
        let mut data_src_hash: HashMap<String, Vec<Port>> = HashMap::new();
        let mut data_dest_hash: HashMap<String, Port> = HashMap::new();
        for (dest, srcs) in sources.clone() {
            if srcs.len() > 1 && get_port_name(&dest).contains("_read_") {
                data_src_hash
                    .insert(get_comp_name(&dest).to_string(), srcs.clone());
                data_dest_hash
                    .insert(get_comp_name(&dest).to_string(), dest.clone());
                let mut structs = srcs
                    .iter()
                    .map(|p| Structure::wire(p.clone(), dest.clone()))
                    .collect();
                changes.batch_remove_structure(&mut structs);
            }
        }
        for (dest, srcs) in sources {
            if srcs.len() > 1
                && !(get_port_name(&dest).contains("_read_")
                    || get_port_name(&dest).starts_with("valid")
                    || get_port_name(&dest).starts_with("ready"))
            {
                let name = self.names.gen_name("lut_data_");
                let mut inputs: Vec<Portdef> = srcs
                    .iter()
                    .map(|_| Portdef {
                        name: self.names.gen_name(&get_port_name(&dest)),
                        width: 32, //XXX: incorrect, should look up input port width
                    })
                    .collect();
                let mut inputs_read: Vec<Portdef> = inputs
                    .iter()
                    .map(|p| Portdef {
                        name: format!("{}_read_in", p.name),
                        width: 1,
                    })
                    .collect();
                inputs.append(&mut inputs_read);
                let mut outputs = vec![Portdef {
                    name: "lut_out".to_string(),
                    width: 32,
                }];
                outputs.push(Portdef {
                    name: "lut_out_read_out".to_string(),
                    width: 1,
                });
                let component = ComponentDef {
                    name: name.clone(),
                    inputs: inputs.clone(),
                    outputs: outputs.clone(),
                    structure: vec![],
                    control: Control::empty(),
                };

                changes.add_structure(Structure::decl(
                    component.name.clone(),
                    component.name.clone(),
                ));
                let mut srcs_all = srcs.clone();
                let srcs_read = match data_src_hash
                    .get(&get_comp_name(&dest).to_string())
                {
                    Some(read_port) => read_port,
                    None => panic!(
                        "cannot find corresonding read port: {}!",
                        &get_comp_name(&dest).to_string()
                    ),
                };
                srcs_all.append(&mut srcs_read.clone());
                for (src, lut_dest) in srcs_all.iter().zip(inputs) {
                    let wire = Structure::wire(
                        src.clone(),
                        Port::Comp {
                            component: name.clone(),
                            port: lut_dest.name,
                        },
                    );
                    changes.add_structure(wire);
                }

                let output_wire_data = Structure::wire(
                    Port::Comp {
                        component: name.clone(),
                        port: outputs[0].clone().name,
                    },
                    dest.clone(),
                );

                let dest_read = match data_dest_hash
                    .get(&get_comp_name(&dest).to_string())
                {
                    Some(read_port) => read_port,
                    None => panic!(
                        "cannot find corresonding read port: {}!",
                        &get_comp_name(&dest).to_string()
                    ),
                };
                let output_wire_read = Structure::wire(
                    Port::Comp {
                        component: name,
                        port: outputs[1].clone().name,
                    },
                    dest_read.clone(),
                );

                changes.add_structure(output_wire_data);
                changes.add_structure(output_wire_read);
                changes.add_component(component);
                let mut structs = srcs
                    .iter()
                    .map(|p| Structure::wire(p.clone(), dest.clone()))
                    .collect();
                changes.batch_remove_structure(&mut structs);
            }
        }

        changes.commit();
        Err(())
    }
}
