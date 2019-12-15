use crate::lang::ast::*;
use crate::passes::visitor::{Changes, Visitor};
use crate::utils::NameGenerator;

pub struct FsmWhile<'a> {
    names: &'a mut NameGenerator,
}

impl FsmWhile<'_> {
    pub fn new(names: &mut NameGenerator) -> FsmWhile {
        FsmWhile { names }
    }
}

impl Visitor<String> for FsmWhile<'_> {
    fn name(&self) -> String {
        "FSM while".to_string()
    }

    fn start_while(
        &mut self,
        c_while: &mut While,
        changes: &mut Changes,
    ) -> Result<(), String> {
        // make input ports for enable fsm component
        let val = Portdef {
            name: "valid".to_string(),
            width: 1,
        };
        let reset = Portdef {
            name: "reset".to_string(),
            width: 1,
        };
        let cond = Portdef {
            name: "condition".to_string(),
            width: 1,
        };
        let clk = Portdef {
            name: "clk".to_string(),
            width: 1,
        };

        //make output ports for enable fsm component
        let rdy = Portdef {
            name: "ready".to_string(),
            width: 1,
        };

        let component_name = self.names.gen_name("fsm_while_");

        let mut inputs: Vec<Portdef> = vec![cond.clone(), val, reset, clk];
        let mut outputs: Vec<Portdef> = vec![rdy];
        let branches = *c_while.body.clone();

        match branches {
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

        let condition_wire = Wire {
            src: c_while.cond.clone(),
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
