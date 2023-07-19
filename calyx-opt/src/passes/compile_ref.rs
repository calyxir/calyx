use super::dump_ports;
use calyx_ir::structure;
use crate::traversal::{
    Action, ConstructVisitor, Named, Order, VisResult, Visitor,
};
use calyx_ir::WRC;
use calyx_ir::{self as ir, LibrarySignatures, RRC};
use calyx_ir::{Attributes, Canonical};
use calyx_utils::{CalyxResult, Error};
use ir::Assignment;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc; 

type PortMap = Vec<(ir::Id, ir::RRC<ir::Port>)>;

/// 1. Remove all the cells marked with the 'ref' keyword
/// 2. Inline all the ports of the ref cells to the component signature
/// 3. Remove all the ref cell mappings from the invoke statement
/// 4. Inline all the mappings of ports to the invoke signature

/// Map for storing added ports for each ref cell
/// level of Hashmap represents:
/// HashMap<-component name-, Hashmap<(-cell name-,-port name-), port>>;
pub(super) type RefPortMap =
    HashMap<ir::Id, HashMap<ir::Canonical, RRC<ir::Port>>>;

pub(super) type GoDonePortMap =
    HashMap<ir::Id, HashMap<ir::Id, Vec<RRC<ir::Port>>>>;

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
    go_ports: GoDonePortMap,
    done_ports: GoDonePortMap
}

impl ConstructVisitor for CompileRef {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized,
    {
        let compile_external = CompileRef {
            port_names: HashMap::new(),
            go_ports: HashMap::new(),
            done_ports: HashMap::new()
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
    // given `ref_cells` of an invoke, reuturns `(inputs, outputs)` where
    // inputs are the corresponding inputs to the `invoke` and
    // outputs are the corresponding outputs to the `invoke`
    fn ref_cells_to_ports(
        &mut self,
        comp_name: ir::Id,
        ref_cells: &mut Vec<(ir::Id, ir::RRC<ir::Cell>)>,
    ) -> (PortMap, PortMap) {
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

        dump_ports::dump_ports_to_signature(
            comp,
            is_external_cell,
            true,
            &mut self.port_names,
            &mut self.go_ports,
            &mut self.done_ports
        );

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap();
        let (mut inputs, mut outputs) =
            self.ref_cells_to_ports(comp_name, &mut s.ref_cells);
        s.inputs.append(&mut inputs);
        s.outputs.append(&mut outputs);

        if s.comp.borrow().is_reference() {
            let mut builder = ir::Builder::new(comp, sigs);
            let invoked_group = builder.add_group("invoke");
            let mut assignments:Vec<Assignment<ir::Nothing>> = Vec::new();
            let c_name = builder.component.name;

            // make assignments for inputs and outputs
            assignments.extend(s.inputs.drain(..).map(|(name, port)| {
                let canon = Canonical(s.comp.borrow().name(), name);
                builder.build_assignment(Rc::clone(&self.port_names[&c_name][&canon]), port, ir::Guard::True)
            }).chain(s.outputs.drain(..).map(|(name, port)| {
                let canon = Canonical(s.comp.borrow().name(), name);
                builder.build_assignment(port, Rc::clone(&self.port_names[&c_name][&canon]), ir::Guard::True)
            })));

            //get go port
            let go = &self.go_ports[&c_name][&s.comp.borrow().name()];

            if go.len() > 1 {
                return Err(Error::malformed_control(format!("Invoked component `{comp_name}` defines multiple @go signals. Cannot compile the invoke")));
            } else if go.is_empty() {
                return Err(Error::malformed_control(format!("Invoked component `{comp_name}` does not define a @go signal. Cannot compile the invoke")));
            }
            let go_p = Rc::clone(&go[0]);

            //get done port
            let done = &self.done_ports[&c_name][&s.comp.borrow().name()];

            if done.len() > 1 {
                return Err(Error::malformed_control(format!("Invoked component `{comp_name}` defines multiple @done signals. Cannot compile the invoke")));
            } else if done.is_empty() {
                return Err(Error::malformed_control(format!("Invoked component `{comp_name}` does not define a @done signal. Cannot compile the invoke")));
            }
            let done_p = Rc::clone(&done[0]);

            //build go assignment and done assignment
            structure!(builder;
                let one = constant(1, 1);
            );

            let go_assign: Assignment<ir::Nothing> = builder.build_assignment(
                go_p,
                one.borrow().get("out"),
                ir::Guard::True,
            );
            let done_assign: Assignment<ir::Nothing> = builder.build_assignment(
                invoked_group.borrow().get("done"),
                done_p,
                ir::Guard::True,
            );

            assignments.push(go_assign);
            assignments.push(done_assign);

            if s.comb_group.is_some() {
                // if invoke has comb group then dump all assignments into group
                let s_owned = std::mem::replace(s, ir::Invoke{
                    comp: Rc::clone(&s.comp),
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    attributes: ir::Attributes::default(),
                    comb_group: None,
                    ref_cells: Vec::new()
                });
                let g_ref = s_owned.comb_group.unwrap();
                let mut g = g_ref.borrow_mut();
                for assignment in std::mem::take(&mut g.assignments) {
                    assignments.push(assignment);
                }
            }

            invoked_group.borrow_mut().assignments.extend(assignments);   
            return Ok(Action::change(ir::Control::Enable(ir::Enable{
                group: invoked_group,
                attributes: Attributes::default()
            })));      
        }
        Ok(Action::Continue)
    }
    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let comp_name = s.comp.borrow().type_name().unwrap();
        let (mut inputs, mut outputs) =
            self.ref_cells_to_ports(comp_name, &mut s.ref_cells);
        s.inputs.append(&mut inputs);
        s.outputs.append(&mut outputs);

        if s.comp.borrow().is_reference() {
            let mut builder = ir::Builder::new(comp, sigs);
            let invoked_group = builder.add_static_group("static_invoke", s.latency);
            let mut assignments:Vec<Assignment<ir::StaticTiming>> = Vec::new();
            let c_name = builder.component.name;

            // make assignments for inputs and outputs
            assignments.extend(s.inputs.drain(..).map(|(name, port)| {
                let canon = Canonical(s.comp.borrow().name(), name);
                builder.build_assignment(Rc::clone(&self.port_names[&c_name][&canon]), port, ir::Guard::True)
            }).chain(s.outputs.drain(..).map(|(name, port)| {
                let canon = Canonical(s.comp.borrow().name(), name);
                builder.build_assignment(port, Rc::clone(&self.port_names[&c_name][&canon]), ir::Guard::True)
            })));

            //get go port
            let go = &self.go_ports[&c_name][&s.comp.borrow().name()];

            if go.len() > 1 {
                return Err(Error::malformed_control(format!("Invoked component `{comp_name}` defines multiple @go signals. Cannot compile the invoke")));
            } else if go.is_empty() {
                return Err(Error::malformed_control(format!("Invoked component `{comp_name}` does not define a @go signal. Cannot compile the invoke")));
            }
            let go_p = Rc::clone(&go[0]);

            structure!(builder;
                let one = constant(1, 1);
            );

            let go_assign: Assignment<ir::StaticTiming> = builder.build_assignment(
                go_p,
                one.borrow().get("out"),
                ir::Guard::True,
            );

            assignments.push(go_assign);

            invoked_group.borrow_mut().assignments.extend(assignments);   

            return Ok(Action::static_change(ir::StaticControl::Enable(ir::StaticEnable{
                group: invoked_group,
                attributes: Attributes::default()
            })));   

        }
        Ok(Action::Continue)
    }
}
