use crate::frontend::library::ast::LibrarySignatures;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::Control;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct WhileSpec;

impl Named for WhileSpec {
    fn name() -> &'static str {
        "while-spec"
    }

    fn description() -> &'static str {
        "Attempts to rewrite while loops to use speculative execution"
    }
}

impl Visitor<()> for WhileSpec {
    fn finish_while(
        &mut self,
        s: &mut ir::While,
        _data: (),
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult<()> {
        let mut builder = ir::Builder::from(comp, ctx, false);
        let ir::While { port, cond, body } = s;

        if let ir::Control::Seq(seq) = &**body {
            if let (
                ir::Control::Enable(enable1),
                ir::Control::Enable(enable2),
                ir::Control::Enable(enable3),
            ) = (&seq.stmts[0], &seq.stmts[1], &seq.stmts[2])
            {
                let enable_a = ir::Control::enable(Rc::clone(&enable1.group));
                let enable_b = ir::Control::enable(Rc::clone(&enable2.group));
                let enable_c = ir::Control::enable(Rc::clone(&enable3.group));

                let a_spec = builder.add_group(
                    enable1.group.borrow().name.to_string() + "_spec",
                    HashMap::new(),
                );
                let b_spec = builder.add_group(
                    enable2.group.borrow().name.to_string() + "_spec",
                    HashMap::new(),
                );
                let c_spec = builder.add_group(
                    enable3.group.borrow().name.to_string() + "_spec",
                    HashMap::new(),
                );

                let enable_a_spec = ir::Control::enable(Rc::clone(&a_spec));
                let enable_b_spec = ir::Control::enable(Rc::clone(&b_spec));
                let enable_c_spec = ir::Control::enable(Rc::clone(&c_spec));

                let commit = builder.add_group("commit_spec", HashMap::new());
                let enable_commit = ir::Control::enable(Rc::clone(&commit));

                let seq1 = Control::seq(vec![enable_b, enable_c]);
                let seq2 = Control::seq(vec![
                    enable_a_spec,
                    enable_b_spec,
                    enable_c_spec,
                ]);
                let par = Control::par(vec![seq1, seq2]);
                let i = Control::if_(
                    Rc::clone(&port),
                    Rc::clone(&cond),
                    Box::new(enable_commit),
                    Box::new(Control::empty()),
                );
                let outer_seq = Control::seq(vec![enable_a, par, i]);

                let w = Control::while_(
                    Rc::clone(&port),
                    Rc::clone(&cond),
                    Box::new(outer_seq),
                );
                return Ok(Action::change_default(w));
            }
        }

        Ok(Action::stop_default())
    }
}

/*

    fn finish_if(
        &mut self,
        cif: &mut ir::If,
        _data: (),
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult<()> {

        const STD_REG_NAME: &str = "std_reg";
        const WIDTH_PARAM: &str = "width";

        // extract group names from control statement
        let (tru, fal) = match (&*cif.tbranch, &*cif.fbranch) {
            (ir::Control::Enable(t), ir::Control::Enable(f)) => {
                Ok((Rc::clone(&t.group), Rc::clone(&f.group)))
            }
            _ => Err(Error::MalformedControl(
                "Both branches of an if must be an enable.".to_string(),
            )),
        }?;

        // collect registers that need to be rewritten
        let mut regs_to_rewrt: HashMap<String, Vec<ir::RRC<ir::Cell>>> = HashMap::new();
        for group_ref in &vec![&tru, &fal] {
            let group = group_ref.borrow();
            let asgns = &group.assignments;
            let group_name = &group.name.to_string();

            regs_to_rewrt.insert(group_name.to_string(), Vec::new());

            for asgn in asgns {
                let parent = &asgn.dst.borrow().parent;
                if let ir::PortParent::Cell( parent_cell ) = parent {

                    if let ir::CellType::Primitive { name, param_binding: _ } = &parent_cell.upgrade().unwrap().borrow().prototype {
                        if *name == STD_REG_NAME {

                            let reg_already_procd = regs_to_rewrt
                                .get(group_name)
                                .unwrap()
                                .iter()
                                .any(|cell_ref| cell_ref.borrow().name == parent_cell.upgrade().unwrap().borrow().name);

                            if !reg_already_procd {
                                regs_to_rewrt
                                    .get_mut(group_name)
                                    .unwrap()
                                    .push(Rc::clone(&parent_cell.upgrade().unwrap()));

                            }
                        }
                    }
                }
            }
        }

        // create shadow registers for each register
        let mut builder = ir::Builder::from(comp, ctx, false);
        let mut rewrites: HashMap<String, Vec<(ir::RRC<ir::Cell>, ir::RRC<ir::Cell>)>> = HashMap::new();

        for (group_name, regs) in regs_to_rewrt {
            rewrites.insert(group_name.clone(), Vec::new());
            for reg in regs {
                let shadow_reg_name = reg.borrow().name.to_string() + &"_".to_owned();
                let c = builder.add_primitive(
                    shadow_reg_name,
                    "std_reg",
                    &[reg.borrow().get_paramter(&ir::Id::from(WIDTH_PARAM)).unwrap()]
                );
                rewrites.get_mut(&group_name).unwrap().push((reg, c));
            }
        }

        // perform renamings
        for group_ref in &vec![&tru, &fal] {
            let mut group = group_ref.borrow_mut();
            let mut assigns = group
                .assignments
                .drain(..)
                .collect::<Vec<_>>();
            builder.rename_port_uses(
                rewrites.get(&group.name.to_string()).unwrap(),
                &mut assigns
            );
            group.assignments = assigns;
        }

        // create commit_true group and commit_false group,
        // which write all of the shadow registers into
        // the real ones
        let cond = Rc::clone(&cif.cond);
        let empty_group = builder.add_group(
            "empty",
            HashMap::new()
        );
        let commit_tru = builder.add_group(
            "commit_".to_owned() + &tru.borrow().name.to_string(),
            HashMap::new()
        );
        let commit_fal = builder.add_group(
            "commit_".to_owned() + &fal.borrow().name.to_string(),
            HashMap::new()
        );

        let mut asgns_commit_tru = Vec::new();
        let mut asgns_commit_fal = Vec::new();
        let mut assigns_empty = Vec::new();

        // create assignments for generated commit groups
        for (group_name, rewrite_pairs) in &rewrites {
            let tru_grp_name = tru.borrow().name.to_string();

            // create shadow -> real port assignments
            for pair in rewrite_pairs {
                let real_in_port = pair.0.borrow().get("in");
                let shadow_out_port = pair.1.borrow().get("out");
                let data_stmt = builder.build_assignment(
                    real_in_port,
                    shadow_out_port,
                    ir::Guard::True
                );

                let one_const = builder.add_constant(1, 1);
                let const_out_port = one_const.borrow().get("out");
                let real_write_en_port = pair.0.borrow().get("write_en");
                let wrt_en_stmt = builder.build_assignment(
                    real_write_en_port,
                    const_out_port,
                    ir::Guard::True
                );

                if group_name == &tru_grp_name {
                    asgns_commit_tru.push(data_stmt);
                    asgns_commit_tru.push(wrt_en_stmt);
                } else {
                    asgns_commit_fal.push(data_stmt);
                    asgns_commit_fal.push(wrt_en_stmt);
                }
            }

            // create done assignments
            let commit_grp_name = "commit_".to_owned() + group_name;

            let commit_grp_done_port = builder
                .component
                .find_group(&commit_grp_name)
                .unwrap()
                .borrow()
                .get("done");

            // pick any done signal from the commit group; the whole
            // group always takes 1 cycle for all done signals to be
            // high
            let reg_done_out_port = rewrites
                .get(group_name)
                .unwrap()[0].0
                .borrow()
                .get("done");

            let wrt_done_stmt = builder.build_assignment(
                commit_grp_done_port,
                reg_done_out_port,
                ir::Guard::True
            );

            if group_name == &tru_grp_name {
                asgns_commit_tru.push(wrt_done_stmt);
            } else {
                asgns_commit_fal.push(wrt_done_stmt);
            }
        }

        // write done port of "empty" group
        let one_const = builder.add_constant(1, 1);
        let const_out_port = one_const.borrow().get("out");
        let empty_done_port = builder
            .component
            .find_group(&"empty".to_owned())
            .unwrap()
            .borrow_mut()
            .get("done");

        let write_empty_stmt = builder
            .build_assignment(
                empty_done_port,
                const_out_port,
                ir::Guard::True
            );

        assigns_empty.push(write_empty_stmt);
        commit_tru.borrow_mut().assignments.append(&mut asgns_commit_tru);
        commit_fal.borrow_mut().assignments.append(&mut asgns_commit_fal);
        empty_group.borrow_mut().assignments.append(&mut assigns_empty);

        let spec = Control::par(vec![
            Control::enable(tru),
            Control::enable(fal),
            Control::enable(cond)
        ]);
        let commit = Control::if_(
            Rc::clone(&cif.port),
            empty_group,
            Box::new(Control::enable(commit_tru)),
            Box::new(Control::enable(commit_fal))
        );
        let result = Control::seq(vec![spec, commit]);

        Ok(Action::change_default(result))
    }
}
*/
