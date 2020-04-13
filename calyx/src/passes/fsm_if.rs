use crate::context::Context;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::passes::visitor::{Action, VisResult, Visitor};
use crate::utils::NameGenerator;

pub struct FsmIf<'a> {
    names: &'a mut NameGenerator,
}

impl FsmIf<'_> {
    pub fn new(names: &mut NameGenerator) -> FsmIf {
        FsmIf { names }
    }
}

impl Visitor for FsmIf<'_> {
    fn name(&self) -> String {
        "FSM if".to_string()
    }

    fn start_if(
        &mut self,
        c_if: &mut ast::If,
        this_comp: &mut Component,
        _ctx: &Context,
    ) -> VisResult {
        // make input ports for enable fsm component
        // let val = ast::Portdef {
        //     name: "valid".to_string(),
        //     width: 1,
        // };
        // let cond = ast::Portdef {
        //     name: "condition".to_string(),
        //     width: 1,
        // };
        // let cond_read = ast::Portdef {
        //     name: "condition_read_in".to_string(),
        //     width: 1,
        // };
        // // make output ports for enable fsm component
        // let rdy = ast::Portdef {
        //     name: "ready".to_string(),
        //     width: 1,
        // };

        let name = self.names.gen_name("fsm_if_");
        let mut fsm_comp = Component::from_signature(
            &name,
            ast::Signature::new(
                &[("valid", 1), ("condition", 1), ("condition_read_in", 1)],
                &[("ready", 1)],
            ),
        );

        // we need to do the same thing for the true and false branch, so we use a for loop
        for (branch, label) in
            [(&*c_if.tbranch, "t"), (&*c_if.fbranch, "f")].iter()
        {
            match branch {
                ast::Control::Enable { data } => {
                    // if data.comps.len() != 1 {
                    //     return Ok(Action::Continue);
                    // }
                    let comp = &data.comps[0];
                    fsm_comp
                        .add_input((format!("ready_{}_{}", label, comp), 1));
                    fsm_comp
                        .add_output((format!("valid_{}_{}", label, comp), 1));
                }
                ast::Control::Empty { .. } => (),
                _ => (), // _ => return Ok(Action::Continue),
            }
        }

        let fsm_node = this_comp.add_instance(&name, &fsm_comp);
        this_comp.add_wire(
            this_comp.get_io_index("valid")?,
            "valid",
            fsm_node,
            "valid",
        )?;
        // XXX(sam) ensure that we need this
        this_comp.add_wire(
            fsm_node,
            "ready",
            this_comp.get_io_index("ready")?,
            "ready",
        )?;

        // let mut inputs: Vec<ast::Portdef> =
        //     vec![cond.clone(), cond_read.clone(), val];
        // let mut outputs: Vec<ast::Portdef> = vec![rdy];

        // add ports for cond
        for id in &c_if.cond {
            // let port_rdy = ast::Portdef {
            //     name: format!("cond_rdy_{}", id),
            //     width: 1,
            // };
            // let port_val = ast::Portdef {
            //     name: format!("cond_val_{}", id),
            //     width: 1,
            // };
            // let ready_wire = ast::Wire {
            //     src: ast::Port::Comp {
            //         component: id.to_string(),
            //         port: "ready".to_string(),
            //     },
            //     dest: ast::Port::Comp {
            //         component: component_name.clone(),
            //         port: port_rdy.name.clone(),
            //     },
            // };
            // let valid_wire = ast::Wire {
            //     src: ast::Port::Comp {
            //         component: component_name.clone(),
            //         port: port_val.name.clone(),
            //     },
            //     dest: ast::Port::Comp {
            //         component: id.to_string(),
            //         port: "valid".to_string(),
            //     },
            // };
            // inputs.push(port_rdy);
            // outputs.push(port_val);
            // changes.add_structure(Structure::Wire { data: ready_wire });
            // changes.add_structure(Structure::Wire { data: valid_wire });
        }

        // let condition_wire = ast::Wire {
        //     src: c_if.port.clone(),
        //     dest: ast::Port::Comp {
        //         component: component_name.clone(),
        //         port: cond.name.clone(),
        //     },
        // };

        // let condition_read_wire = ast::Wire {
        //     src: match &c_if.port {
        //         ast::Port::This { .. } => ast::Port::This {
        //             port: "out_read_out".to_string(),
        //         },
        //         ast::Port::Comp { component, .. } => ast::Port::Comp {
        //             component: component.to_string(),
        //             port: "out_read_out".to_string(),
        //         },
        //     },
        //     dest: ast::Port::Comp {
        //         component: component_name.clone(),
        //         port: cond_read.name.clone(),
        //     },
        // };

        // changes.add_structure(Structure::Wire {
        //     data: condition_read_wire,
        // });
        // changes.add_structure(Structure::Wire {
        //     data: condition_wire,
        // });

        // let component = ast::ComponentDef {
        //     name: component_name.clone(),
        //     signature: ast::Signature { inputs, outputs },
        //     structure: vec![],
        //     control: ast::Control::empty(),
        // };

        // changes.add_structure(Structure::decl(
        //     component.name.clone(),
        //     component.name.clone(),
        // ));

        // changes.add_component(component);
        // changes.change_node(Control::enable(vec![component_name]));
        // changes.commit();
        Ok(Action::Continue)
    }
}
