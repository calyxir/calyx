use crate::lang::ast::*;
use crate::passes::visitor::{Changes, Visitor};
use crate::utils::combine;

pub struct FsmList {}

impl Visitor<()> for FsmList {
    fn new() -> FsmList {
        FsmList {}
    }

    fn name(&self) -> String {
        "FSM enable".to_string()
    }

    fn start_enable(
        &mut self,
        en: &mut Enable,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        if en.comps.len() > 1 {
            // make input ports for enable fsm component
            let val = Portdef {
                name: "valid".to_string(),
                width: 32,
            };
            let reset = Portdef {
                name: "reset".to_string(),
                width: 32,
            };

            // make output ports for enable fsm component
            let rdy = Portdef {
                name: "ready".to_string(),
                width: 32,
            };

            let component_name =
                format!("enable_fsm_{}", combine(&en.comps, "_", ""));

            // generate ports and wires from enabled components
            let mut inputs: Vec<Portdef> = vec![val, reset];
            let mut outputs: Vec<Portdef> = vec![rdy];
            for comp in &en.comps {
                let ready = Portdef {
                    name: format!("ready_{}", comp),
                    width: 32,
                };
                let valid = Portdef {
                    name: format!("valid_{}", comp),
                    width: 32,
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

            let component = Component {
                name: component_name,
                inputs,
                outputs,
                structure: vec![],
                control: Control::empty(),
            };

            changes.add_structure(Structure::decl(
                component.name.clone(),
                "enable_fsm".to_string(),
            ));

            // change the instruction
            en.comps = vec![component.name.clone()];

            changes.add_component(component);
        }

        Ok(())
    }
}
