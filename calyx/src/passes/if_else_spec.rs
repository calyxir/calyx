use crate::errors::Error;
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{Component, Control};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use crate::analysis;

/// Transforms if/else statements of this form:
///
///    if port with cond {
///      a;
///    else {
///      b;
///    }
///
/// to:
///
///    seq {
///      par {
///        a_aux;
///        b_aux;
///        cond;
///      }
///
///      if port with cond {
///        commit_a;
///      else {
///        commit_b;
///      }
///    }
///
/// a_aux and b_aux are a and b rewritten, with all registers
/// that are written to replaced with temporary registers.
/// commit_a and commit_b each write these temporary registers
/// to the original registers in 1 cycle.
#[derive(Default)]
pub struct IfElseSpec;

impl Named for IfElseSpec {
    fn name() -> &'static str {
        "if_else_spec"
    }

    fn description() -> &'static str {
        "Rewrites if/else statements to parallelize execution of each branch, and commit results only if needed"
    }
}

impl Visitor<()> for IfElseSpec {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        // This pass doesn't modify any control.
        Ok(Action::continue_default())
    }

    fn finish_if(
        &mut self,
        cif: &mut ir::If,
        _data: (),
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult<()> {

        //Create new registers 
        /// For each group, find each register that's written to, and push that pair to list.
        let mut rewrites: HashMap<String, Vec<ir::RRC<ir::Cell>>> = HashMap::new();

        for group in &mut comp.groups {
            rewrites.insert(group.borrow().name.to_string(), Vec::new());
            for asgn in &group.borrow().assignments {
                if let ir::PortParent::Cell( c ) = &asgn.dst.borrow().parent {
                    // TODO: dont add groups, make sure this is _just_ registers, instead of hardcoding these
                    if c.upgrade().unwrap().borrow().name != "a" && c.upgrade().unwrap().borrow().name != "b" && c.upgrade().unwrap().borrow().name != "lt" && !c.upgrade().unwrap().borrow().name.to_string().starts_with("c") {
                        if !rewrites.get(&group.borrow().name.to_string()).unwrap().iter().any(|c2| c2.borrow().name == c.upgrade().unwrap().borrow().name) {
                            rewrites.get_mut(&group.borrow().name.to_string()).unwrap().push(Rc::clone(&c.upgrade().unwrap()));
                        }
                    }
                }
            }
        }

        let mut builder = ir::Builder::from(comp, ctx, false);


        let mut rewrites2: HashMap<String, Vec<(ir::RRC<ir::Cell>, ir::RRC<ir::Cell>)>> = HashMap::new();
        for (gname, regs) in rewrites {
            rewrites2.insert(gname.clone(), Vec::new());
            for reg in regs {
                let c = builder.add_primitive(reg.borrow().name.to_string() + &"_".to_owned(), "std_reg", &vec![1]);
                rewrites2.get_mut(&gname).unwrap().push((reg, c));
            }
        }

        for group_ref in &builder.component.groups {
            let mut group = group_ref.borrow_mut();
            if &group.name != "cond" {
                let mut assigns = group.assignments.drain(..).collect::<Vec<_>>();
                builder.rename_port_uses(rewrites2.get(&group.name.to_string()).unwrap(), &mut assigns);
                group.assignments = assigns;
            }
        }
        
        // extract group names from control statement
        let (tru, fal) = match (&*cif.tbranch, &*cif.fbranch) {
            (ir::Control::Enable(t), ir::Control::Enable(f)) => {
                Ok((Rc::clone(&t.group), Rc::clone(&f.group)))
            }
            _ => Err(Error::MalformedControl(
                "Both branches of an if must be an enable.".to_string(),
            )),
        }?;

        // Collect all registers written to in a, and b.
        // Change all register names by appending a "_".

        // Create commit_a group and commit_b group, which just write all the registers we found in a and b.

        let cond = Rc::clone(&cif.cond);
        let empty_group = builder.add_group("empty", HashMap::new());
        let commit_a = builder.add_group("commit_".to_owned() + &tru.borrow().name.to_string(), HashMap::new());
        let commit_b = builder.add_group("commit_".to_owned() + &fal.borrow().name.to_string(), HashMap::new());

        let mut assigns_a = Vec::new();
        let mut assigns_b = Vec::new();
        let mut assigns_empty = Vec::new();
        for (gname, regpairs) in &rewrites2 {
            for regpair in regpairs {
                let c = builder.add_constant(1, 1);
                if gname == "a" {
                    assigns_a.push(builder.build_assignment(regpair.0.borrow().get("in"), regpair.1.borrow().get("out"), ir::Guard::True));
                    assigns_a.push(builder.build_assignment(regpair.0.borrow().get("write_en"), c.borrow().get("out"), ir::Guard::True));
                }

                if gname == "b" {
                    assigns_b.push(builder.build_assignment(regpair.0.borrow().get("in"), regpair.1.borrow().get("out"), ir::Guard::True));
                    assigns_b.push(builder.build_assignment(regpair.0.borrow().get("write_en"), c.borrow().get("out"), ir::Guard::True));
                }
            }
        }

        let b = builder.add_constant(1, 1);
        assigns_empty.push(builder.build_assignment(builder.component.find_group(&"empty".to_owned()).unwrap().borrow_mut().get("done"), b.borrow().get("out"), ir::Guard::True));

        assigns_a.push(builder.build_assignment(
            builder.component.find_group(&"commit_a".to_owned()).unwrap().borrow().get("done"),
            rewrites2.get("a").unwrap()[0].0.borrow().get("done"),
            ir::Guard::True)
        );

        assigns_b.push(builder.build_assignment(
            builder.component.find_group(&"commit_b".to_owned()).unwrap().borrow().get("done"),
            rewrites2.get("b").unwrap()[0].0.borrow().get("done"),
            ir::Guard::True)
        );

        commit_a.borrow_mut().assignments.append(&mut assigns_a);
        commit_b.borrow_mut().assignments.append(&mut assigns_b);
        empty_group.borrow_mut().assignments.append(&mut assigns_empty);

        let spec = Control::par(vec![Control::enable(tru), Control::enable(fal), Control::enable(cond)]);
        let commit = Control::if_(Rc::clone(&cif.port), empty_group, Box::new(Control::enable(commit_a)), Box::new(Control::enable(commit_b)));
        let result = Control::seq(vec![spec, commit]);
        
        Ok(Action::change_default(result))

    }
}