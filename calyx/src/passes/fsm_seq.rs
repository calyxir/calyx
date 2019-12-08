use crate::lang::ast::*;
use crate::passes::visitor::{Changes, Visitor};
use crate::utils::NameGenerator;

pub struct FsmSeq<'a> {
    names: &'a mut NameGenerator,
}

impl FsmSeq<'_> {
    pub fn new(names: &mut NameGenerator) -> FsmSeq {
        FsmSeq { names }
    }
}

impl Visitor<String> for FsmSeq<'_> {
    fn name(&self) -> String {
        "FSM seq".to_string()
    }

    fn start_seq(
        &mut self,
        seq: &mut Seq,
        changes: &mut Changes,
    ) -> Result<(), String> {
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

        let component_name = self.names.gen_name("fsm_seq_");

        let mut inputs: Vec<Portdef> = vec![val, reset];
        let mut outputs: Vec<Portdef> = vec![rdy];

        for con in &mut seq.stmts {
            match con {
                Control::Enable { data } => {
                    if data.comps.len() != 1 {
                        return Ok(());
                    }
                    let comp = &data.comps[0];
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
                Control::Empty { .. } => (),
                _x => return Ok(()),
            }
        }

        let component = Component {
            name: component_name.clone(),
            inputs,
            outputs,
            structure: vec![],
            control: Control::empty(),
        };

        changes.add_structure(Structure::decl(
            component.name.clone(),
            "fsm_seq".to_string(),
        ));

        changes.add_component(component);
        changes.change_node(Control::enable(vec![component_name]));
        Ok(())
    }
}
