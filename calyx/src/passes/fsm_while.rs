use crate::lang::ast::*;
use crate::passes::visitor::{Changes, Visitor};
use crate::utils::NameGenerator;

pub struct FsmWhile{
	unique: NameGenerator,
}

impl Visitor<()> for FsmWhile {
	fn new() -> FsmWhile {
		FsmWhile {
			unique: NameGenerator::new(),
		}
	}

	fn name(&self) -> String {
		"FSM while".to_string()
	}

	fn start_while(
		&mut self,
		c_while: &mut While,
		changes: &mut Changes,
	) -> Result<(), ()> {
		// make input ports for enable fsm component
		let val = Portdef {
			name: "valid".to_string(),
			width: 32,
		};
		let reset = Portdef {
			name: "reset".to_string(),
			width: 32,
		};
		let cond = Portdef {
			name: "condition".to_string(),
			width: 32,
		};

		//make output ports for enable fsm component
		let rdy = Portdef {
			name: "ready".to_string(),
			width: 32
		};

		let component_name = self.unique.gen_name("fsm_while_");

		let mut inputs: Vec<Portdef> = vec![cond.clone(), val, reset];
		let mut outputs: Vec<Portdef> = vec![rdy];
		let mut branches = *c_while.body.clone();

		match branches {
			Control::Enable { data } => {
				if data.comps.len() != 1 {
					return Err(());
				}
				let comp = &data.comps[0];
				let ready = Portdef {
					name: format!("ready_{}", comp),
					width: 32
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
                //data.comps = vec![component_name.clone()];
			}
			Control::Empty { .. } => (),
            _ => return Err(()),
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
			"fsm_while".to_string(),
		));

		changes.add_component(component);
		changes.change_node(Control::enable(vec![component_name]));
        Ok(())
	}
}