use super::dump_ports;
use crate::errors::CalyxResult;
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};
use crate::ir::Attributes;
use crate::ir::WRC;
use crate::ir::{self, LibrarySignatures, RRC};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct CompileRef {
    port_names: HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, ir::Id>>>,
}

impl ConstructVisitor for CompileRef {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let compile_external = CompileRef {
            port_names: HashMap::new(),
        };
        Ok(compile_external)
    }

    fn clear_data(&mut self) {
        //data is shared between components
    }
}

fn is_external_cell(cr: &RRC<ir::Cell>) -> bool {
    cr.borrow().is_reference()
}

impl Named for CompileRef {
    fn name() -> &'static str {
        "compile-ref"
    }

    fn description() -> &'static str {
        "Inline the ports of reference cells to component signature"
    }
}

impl Visitor for CompileRef {
    fn require_postorder() -> bool {
        true
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        dump_ports::dump_ports_to_signature(
            comp,
            is_external_cell,
            &mut self.port_names,
        );
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap().clone();
        for (id, cell) in s.external_cells.drain(..) {
            for port in cell.borrow().ports.iter() {
                if port.borrow().attributes.get("clk").is_none()
                    && port.borrow().attributes.get("reset").is_none()
                {
                    if matches!(port.borrow().direction, ir::Direction::Input) {
                        let port_name = self.port_names[&comp_name][&id]
                            [&port.borrow().name.clone()]
                            .clone();
                        s.outputs.push((port_name.clone(), Rc::clone(port)));
                        let p = Rc::new(RefCell::new(ir::Port {
                            name: port_name.clone(),
                            width: port.borrow().width,
                            direction: ir::Direction::Output,
                            parent: ir::PortParent::Cell(WRC::from(&s.comp)),
                            attributes: Attributes::default(),
                        }));
                        s.comp.borrow_mut().ports.push(p);
                    } else if matches!(
                        port.borrow().direction,
                        ir::Direction::Output
                    ) {
                        let port_name = self.port_names[&comp_name][&id]
                            [&port.borrow().name.clone()]
                            .clone();
                        s.inputs.push((port_name.clone(), Rc::clone(port)));
                        let p = Rc::new(RefCell::new(ir::Port {
                            name: port_name.clone(),
                            width: port.borrow().width,
                            direction: ir::Direction::Input,
                            parent: ir::PortParent::Cell(WRC::from(&s.comp)),
                            attributes: Attributes::default(),
                        }));
                        s.comp.borrow_mut().ports.push(p);
                    }
                }
            }
        }
        Ok(Action::Continue)
    }
}
