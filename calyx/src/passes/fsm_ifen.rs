use crate::lang::ast::*;
use crate::passes::visitor::{Changes, Visitor};
use crate::utils::NameGenerator;

pub struct FsmIfen<'a> {
    names: &'a mut NameGenerator,
}

impl FsmIfen<'_> {
    pub fn new(names: &mut NameGenerator) -> FsmIfen {
        FsmIfen { names }
    }
}

impl Visitor<()> for FsmIfen<'_> {
    fn name(&self) -> String {
        "FSM ifen".to_string()
    }

    fn start_ifen(
        &mut self,
        ifen: &mut Ifen,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        // make input ports for enable fsm component
        let val = Portdef {
            name: "valid".to_string(),
            width: 1,
        };
        let cond = Portdef {
            name: "condition".to_string(),
            width: 1,
        };
        // make output ports for enable fsm component
        let rdy = Portdef {
            name: "ready".to_string(),
            width: 1,
        };

        let component_name = self.names.gen_name("fsm_ifen_");

        let mut inputs: Vec<Portdef> = vec![cond.clone(), val];
        let mut outputs: Vec<Portdef> = vec![rdy];
        let mut branchs = vec![*ifen.tbranch.clone(), *ifen.fbranch.clone()];
        for con in &mut branchs {
            match con {
                Control::Enable { data } => {
                    if data.comps.len() != 1 {
                        return Ok(());
                    }
                    let comp = &data.comps[0];
                    let ready = Portdef {
                        name: format!("ready_{}", comp),
                        width: 1,
                    };
                    let valid = Portdef {
                        name: format!("valid_{}", comp),
                        width: 1,
                    };
                    let ready_wire = Wire {
                        src: Port::Comp {
                            component: comp.to_string(),
                            port: "ready".to_string(),
                        },
                        dest: Port::Comp {
                            component: component_name.clone(),
                            port: ready.name.clone(),
                        },
                    };
                    let valid_wire = Wire {
                        src: Port::Comp {
                            component: component_name.clone(),
                            port: valid.name.clone(),
                        },
                        dest: Port::Comp {
                            component: comp.to_string(),
                            port: "valid".to_string(),
                        },
                    };
                    inputs.push(ready);
                    outputs.push(valid);
                    changes.add_structure(Structure::Wire { data: ready_wire });
                    changes.add_structure(Structure::Wire { data: valid_wire });
                }
                Control::Empty { .. } => (),
                _ => return Ok(()),
            }
        }

        let condition_wire = Wire {
            src: ifen.port.clone(),
            dest: Port::Comp {
                component: component_name.clone(),
                port: cond.name.clone(),
            },
        };

        changes.add_structure(Structure::Wire {
            data: condition_wire,
        });

        let component = Component {
            name: component_name.clone(),
            inputs,
            outputs,
            structure: vec![],
            control: Control::empty(),
        };

        changes.add_structure(Structure::decl(
            component.name.clone(),
            component.name.clone(),
        ));

        changes.add_component(component);
        changes.change_node(Control::enable(vec![component_name]));
        Ok(())
    }
}
