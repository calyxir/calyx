use super::dump_ports;
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::WRC;
use calyx_ir::{self as ir, LibrarySignatures, RRC};
use calyx_ir::{Attributes, Canonical};
use calyx_utils::CalyxResult;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// 1. Remove all the cells marked with the 'ref' keyword
/// 2. Inline all the ports of the ref cells to the component signature
/// 3. Remove all the ref cell mappings from the invoke statement
/// 4. Inline all the mappings of ports to the invoke signature

/// Map for storing added ports for each ref cell
/// level of Hashmap represents:
/// HashMap<-component name-, Hashmap<(-cell name-,-port name-), port>>;
pub(super) type RefPortMap =
    HashMap<ir::Id, HashMap<ir::Canonical, RRC<ir::Port>>>;

trait GetPorts {
    fn get_ports(&self, comp_name: &ir::Id) -> Option<Vec<RRC<ir::Port>>>;
}

impl GetPorts for RefPortMap {
    fn get_ports(&self, comp_name: &ir::Id) -> Option<Vec<RRC<ir::Port>>> {
        if self.contains_key(comp_name) {
            let mut ret = Vec::new();
            for (_, p) in self[comp_name].iter() {
                ret.push(Rc::clone(p));
            }
            Some(ret)
        } else {
            None
        }
    }
}
pub struct CompileRef {
    port_names: RefPortMap,
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
        // data is shared between components
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
        "Inline the ports of reference cells to component signature and the invoke signature"
    }
}

impl CompileRef {
    fn ref_cells_to_ports(
        &mut self,
        comp_name: ir::Id,
        ref_cells: &mut Vec<(ir::Id, ir::RRC<ir::Cell>)>,
    ) -> (
        Vec<(ir::Id, ir::RRC<ir::Port>)>,
        Vec<(ir::Id, ir::RRC<ir::Port>)>,
    ) {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for (in_cell, cell) in ref_cells.drain(..) {
            for port in cell.borrow().ports.iter() {
                if port.borrow().attributes.get(ir::BoolAttr::Clk).is_none()
                    && port
                        .borrow()
                        .attributes
                        .get(ir::BoolAttr::Reset)
                        .is_none()
                {
                    let canon = Canonical(in_cell, port.borrow().name);
                    let port_name =
                        self.port_names[&comp_name][&canon].borrow().name;
                    match port.borrow().direction {
                        ir::Direction::Input => {
                            outputs.push((port_name, Rc::clone(port)));
                        }
                        ir::Direction::Output => {
                            inputs.push((port_name, Rc::clone(port)));
                        }
                        _ => {
                            unreachable!("Internal Error: This state should not be reachable.");
                        }
                    }
                }
            }
        }
        (inputs, outputs)
    }
}

impl Visitor for CompileRef {
    fn iteration_order() -> Order {
        Order::Post
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
            true,
            &mut self.port_names,
        );

        for cell in comp.cells.iter() {
            let mut new_ports: Vec<RRC<ir::Port>> = Vec::new();
            if let Some(ref name) = cell.borrow().type_name() {
                if let Some(vec) = self.port_names.get_ports(name) {
                    for p in vec.iter() {
                        let new_port = Rc::new(RefCell::new(ir::Port {
                            name: p.borrow().name,
                            width: p.borrow().width,
                            direction: p.borrow().direction.reverse(),
                            parent: ir::PortParent::Cell(WRC::from(cell)),
                            attributes: Attributes::default(),
                        }));
                        new_ports.push(new_port);
                    }
                }
            }
            cell.borrow_mut().ports.extend(new_ports);
        }
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap();
        let (mut inputs, mut outputs) =
            self.ref_cells_to_ports(comp_name, &mut s.ref_cells);
        s.inputs.append(&mut inputs);
        s.outputs.append(&mut outputs);
        Ok(Action::Continue)
    }
    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap();
        let (mut inputs, mut outputs) =
            self.ref_cells_to_ports(comp_name, &mut s.ref_cells);
        s.inputs.append(&mut inputs);
        s.outputs.append(&mut outputs);
        Ok(Action::Continue)
    }
}
