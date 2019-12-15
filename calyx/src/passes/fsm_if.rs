use crate::lang::ast::*;
use crate::passes::visitor::{Changes, Visitor};
use crate::utils::NameGenerator;

pub struct FsmIf<'a> {
    names: &'a mut NameGenerator,
}

impl FsmIf<'_> {
    pub fn new(names: &mut NameGenerator) -> FsmIf {
        FsmIf { names }
    }
}

impl Visitor<()> for FsmIf<'_> {
    fn name(&self) -> String {
        "FSM if".to_string()
    }

    fn start_if(
        &mut self,
        c_if: &mut If,
        changes: &mut Changes,
    ) -> Result<(), ()> {
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

        // make output ports for enable fsm component
        let rdy = Portdef {
            name: "ready".to_string(),
            width: 1,
        };

        let component_name = self.names.gen_name("fsm_if_");

        let mut inputs: Vec<Portdef> = vec![cond.clone(), val, reset, clk];
        let mut outputs: Vec<Portdef> = vec![rdy];
        let mut branchs = vec![*c_if.tbranch.clone(), *c_if.fbranch.clone()];
        println!("{:#?}", branchs);
        let mut i = 0;
        for con in &mut branchs {
            match con {
                Control::Enable { data } => {
                    if data.comps.len() != 1 {
                        return Ok(());
                    }
                    let comp = &data.comps[0];
                    let ready = if i == 0 {
                        Portdef {
                            name: format!("ready_t_{}", comp),
                            width: 1,
                        }
                    } else {
                        Portdef {
                            name: format!("ready_f_{}", comp),
                            width: 1,
                        }
                    };
                    let valid = if i == 0 {
                        Portdef {
                            name: format!("valid_t_{}", comp),
                            width: 1,
                        }
                    } else {
                        Portdef {
                            name: format!("valid_f_{}", comp),
                            width: 1,
                        }
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
                    let clk_wire = Wire {
                        src: Port::This {
                            port: "clk".to_string(),
                        },
                        dest: Port::Comp {
                            component: component_name.clone(),
                            port: "clk".to_string(),
                        },
                    };
                    let reset_wire = Wire {
                        src: Port::This {
                            port: "reset".to_string(),
                        },
                        dest: Port::Comp {
                            component: component_name.clone(),
                            port: "reset".to_string(),
                        },
                    };
                    inputs.push(ready);
                    outputs.push(valid);
                    changes.add_structure(Structure::Wire { data: ready_wire });
                    changes.add_structure(Structure::Wire { data: valid_wire });
                    changes.add_structure(Structure::Wire { data: clk_wire });
                    changes.add_structure(Structure::Wire { data: reset_wire });
                }
                Control::Empty { data } => {
                    println!("{:?}", data);
                }
                _ => return Ok(()),
            }
            i += 1;
        }

        let condition_wire = Wire {
            src: c_if.cond.clone(),
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
