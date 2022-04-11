use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor}, RRC,
};
use std::{collections::HashMap};
use itertools::partition;
use crate::ir::Enable;
use std::rc::Rc;
use crate::ir::Attributes;


/// Pass to do something
#[derive(Default)]
pub struct MergeStaticPar ;

impl Named for MergeStaticPar {
    fn name() -> &'static str {
        "merge-static-par"
    }

    fn description() -> &'static str {
        "merge static pars when they have the same static time"
    }
}

impl Visitor for MergeStaticPar {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut static_group:HashMap<u64, Vec<RRC<ir::Group>>> = HashMap::new();
        let idx = partition(&mut s.stmts, 
            |stmt| matches!(stmt, ir::Control::Enable(_)));

        let e_stmts: Vec<_> = s.stmts.drain(0..idx).collect();


        for stmt in e_stmts {
            //let mut err = std::io::stderr();
            //ir::Printer::write_control(stmt, 0, &mut err)?;
             

            if let ir::Control::Enable(data) = stmt {
                let group = &data.group;
                let static_time: u64 =
                    *group.borrow().attributes.get("static").unwrap();
                static_group.entry(static_time).or_default().push(Rc::clone(group));      
            }
        }

            for (key, value) in static_group {
                if value.len() != 1 {
                    let mut builder = ir::Builder::new(_comp, _sigs);
                    let grp = builder.add_group("msp");
                    let mut assignments : Vec<ir::Assignment> = Vec::new(); 
                    for group in value.iter() {
                        for asmt in group.borrow().assignments.clone() {
                        assignments.push(asmt);
                        }
                    }

                    let idx = partition(&mut assignments, 
                        |x| x.dst.borrow().is_hole());
                    let done_asmts: Vec<_> = assignments.drain(0..idx).collect();

                    for asmt in assignments {
                        grp.borrow_mut().assignments.push(asmt);
                    } 

                    let mut ports: Vec<ir::Guard> = Vec::new(); 
                    for asmt in done_asmts.clone() {
                        let grd: ir::Guard = ir::Guard::Port(asmt.src);
                        ports.push(grd);
                        ports.push(*asmt.guard);
                    }
                
                    let mut fin_grd: ir::Guard = ir::Guard::True;
                    for grd in ports {
                        fin_grd = fin_grd & grd;
                    }

                    //let grp_mut: &mut ir::Group = &mut *grp.borrow_mut();

                    let cst = builder.add_constant(1, 1);

                    let done_asmt = 
                    builder.build_assignment( 
                    grp.borrow().get("done"), 
                    cst.borrow().get("out"),
                    fin_grd);

                    {grp.borrow_mut().assignments.push(done_asmt);}
                    grp.borrow_mut().attributes.insert("static", key);
                    _comp.groups.add(Rc::clone(&grp));
                

                    let enable : ir::Enable = Enable{
                        group: Rc::clone(&grp),
                        attributes: Attributes::default(),
                    };
                    s.stmts.push(ir::Control::Enable(enable));
                }

                else {
                    let enable: ir::Enable = Enable { 
                        group: Rc::clone(&value[0]), attributes: Attributes::default()};
                    s.stmts.push(ir::Control::Enable(enable));
                }

            
            } 
        

        
        Ok(Action::Continue)
    }
}
