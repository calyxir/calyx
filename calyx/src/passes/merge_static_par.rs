use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use std::collections::HashMap;
use itertools::partition;
use crate::ir::Enable;
use std::rc::Rc;
use std::cell::RefCell;
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
        let mut static_group:HashMap<u64, Vec<ir::Group>> = HashMap::new();
        let (e_stmts, n_stmts) = partition(&mut s.stmts, 
            |stmt| ir::Control::Enable(_) == stmt);

        s.stmts = Vec::new();

        for stmt in n_stmts.iter() {
            s.stmts.push(stmt);
        }


        for stmt in e_stmts.iter() {
            //let mut err = std::io::stderr();
            //ir::Printer::write_control(stmt, 0, &mut err)?;
             

            if let ir::Control::Enable(data) = stmt {
                let group = &data.group;
                let static_time: u64 =
                    *group.borrow().attributes.get("static").unwrap();
                if !static_group.contains_key(&static_time) {
                    static_group.insert(static_time, Vec::new());
                }
                let mut group_vec: Vec<ir::Group> = static_group.get(&static_time);
                group_vec.push(*group.borrow());         
            }

            for (key, value) in static_group {
                if value.len() != 1 {
                    let mut builder = ir::Builder::new(_comp);
                    let mut grp = builder.add_group();
                    let mut assignments : Vec<ir::Assignment> = Vec::new(); 
                    for group in value.iter() {
                        for asmt in *group.borrow().assignments {
                        assignments.push(asmt);
                        }
                    }

                    let (n_asmts, done_asmts) = partition(&mut assignments, 
                        |x| &(x.dst.borrow()).is_hole() && x.attributes.has("done"));

                    for asmt in n_asmts.iter() {
                        grp.assignments.push(asmt);
                    } 

                    let mut ports: Vec<ir::Guard> = Vec::new(); 
                    for asmt in done_asmts.iter() {
                        let mut grd: ir::Guard = ir::Guard::Port(asmt.src);
                        ports.push(grd);
                        ports.push(*asmt.guard);
                    }
                
                    let mut fin_grd: ir::Guard = ir::Guard::True;
                    for grd in ports.iter() {
                        fin_grd = fin_grd & grd;
                    }
                
                    let mut done_asmt = grp.done_cond_mut();
                    *done_asmt.src = *value[0].done_cond().src;
                    *done_asmt.guard = Box::new(fin_grd);
                

                    let enable : ir::Enable = Enable{
                        group: Rc::new(RefCell::new(grp)),
                        attributes: Attributes::new(),
                    };
                    s.stmts.push(ir::Control::Enable(enable));
                }

                else {
                    s.stmts.push(stmt);
                }

            
            } 
        }

        
        Ok(Action::Continue)
    }
}
