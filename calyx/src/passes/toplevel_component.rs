use crate::lang::ast::{ComponentDef, Control, Id, Port, Structure, Wire};
use crate::passes::visitor::{Changes, Visitor};

pub struct Toplevel {
    name: Id,
}

impl Toplevel {
    pub fn new(name: Id) -> Self {
        Toplevel { name }
    }
}

impl Visitor<()> for Toplevel {
    fn name(&self) -> String {
        "Pass to hook up toplevel interface".to_string()
    }

    fn start(
        &mut self,
        component: &mut ComponentDef,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        if component.name == self.name {
            match &component.control {
                Control::Enable { data } => {
                    if data.comps.len() != 1 {
                        panic!("Expected the enable to only have a single component")
                    }

                    let enabled_comp = &data.comps[0];

                    let valid_wire = Wire {
                        src: Port::This {
                            port: "valid".to_string(),
                        },
                        dest: Port::Comp {
                            component: enabled_comp.to_string(),
                            port: "valid".to_string(),
                        },
                    };

                    changes.add_structure(Structure::Wire { data: valid_wire });
                }
                _ => panic!("Expected enable in the toplevel component"),
            }
        }

        // return err to avoid recursing down the whole tree
        changes.commit();
        Err(())
    }
}
