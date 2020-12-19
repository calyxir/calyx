use crate::analysis::ReadWriteSet;
use crate::frontend::library::ast::LibrarySignatures;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::Control;
use std::collections::{HashMap, HashSet};
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
        const WIDTH_PARAM: &str = "width";
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

                let mut a_asgns =
                    enable1.group.borrow().assignments.clone().to_vec();
                let mut b_asgns =
                    enable2.group.borrow().assignments.clone().to_vec();
                let mut c_asgns =
                    enable3.group.borrow().assignments.clone().to_vec();

                a_spec.borrow_mut().assignments.append(&mut a_asgns);
                b_spec.borrow_mut().assignments.append(&mut b_asgns);
                c_spec.borrow_mut().assignments.append(&mut c_asgns);

                let enable_a_spec = ir::Control::enable(Rc::clone(&a_spec));
                let enable_b_spec = ir::Control::enable(Rc::clone(&b_spec));
                let enable_c_spec = ir::Control::enable(Rc::clone(&c_spec));

                let mut reg_to_buffer: Vec<(
                    ir::RRC<ir::Cell>,
                    ir::RRC<ir::Cell>,
                )> = Vec::new();
                // - If a group writes to a register, and the other groups read from that value,
                //   make a buffer register.
                // - If a group reads a register, and the other groups write to that register,
                //   make a buffer register. (Otherwise, fine to use the original register.)
                let orig_groups =
                    vec![&enable1.group, &enable2.group, &enable3.group];
                let spec_groups = vec![a_spec, b_spec, c_spec];
                for (i, grp) in orig_groups.iter().enumerate() {
                    let grp_writes =
                        ReadWriteSet::write_set(&grp.borrow().assignments);
                    let grp_reads =
                        ReadWriteSet::read_set(&grp.borrow().assignments);

                    let mut other_reads: Vec<ir::RRC<ir::Cell>> = Vec::new();
                    for other_grp in &orig_groups {
                        if grp.borrow().name != other_grp.borrow().name {
                            let mut other_grp_reads = ReadWriteSet::read_set(
                                &other_grp.borrow().assignments,
                            )
                            .clone();
                            other_reads.append(&mut other_grp_reads);
                        }
                    }

                    let mut other_writes: Vec<ir::RRC<ir::Cell>> = Vec::new();
                    for other_grp in &orig_groups {
                        if !Rc::ptr_eq(&grp, &other_grp) {
                            let mut other_grp_writes = ReadWriteSet::write_set(
                                &other_grp.borrow().assignments,
                            )
                            .clone();
                            other_writes.append(&mut other_grp_writes)
                        }
                    }

                    let mut wrt_to_buf: Vec<(
                        ir::RRC<ir::Cell>,
                        ir::RRC<ir::Cell>,
                    )> = Vec::new();
                    for wrt in grp_writes {
                        if other_reads
                            .iter()
                            .any(|c| c.borrow().name == wrt.borrow().name)
                        {
                            let buf_reg =
                                builder.component.cells.iter().find(|c| {
                                    c.borrow().name
                                        == (wrt.borrow().name.to_string()
                                            + "_spec")
                                });
                            match buf_reg {
                                Some(reg) => {
                                    reg_to_buffer
                                        .push((wrt.clone(), Rc::clone(&reg)));
                                    wrt_to_buf
                                        .push((wrt.clone(), Rc::clone(&reg)));
                                }
                                None => {
                                    let new_buf_reg = builder.add_primitive(
                                        wrt.borrow().name.to_string() + "_spec",
                                        "std_reg",
                                        &[wrt
                                            .borrow()
                                            .get_paramter(&ir::Id::from(
                                                WIDTH_PARAM,
                                            ))
                                            .unwrap()],
                                    );

                                    reg_to_buffer.push((
                                        wrt.clone(),
                                        new_buf_reg.clone(),
                                    ));
                                    wrt_to_buf.push((
                                        wrt.clone(),
                                        new_buf_reg.clone(),
                                    ));
                                }
                            }
                        }
                    }

                    let mut read_to_buf = Vec::new();
                    for read in grp_reads {
                        if other_writes
                            .iter()
                            .any(|c| c.borrow().name == read.borrow().name)
                        {
                            let buf_reg =
                                builder.component.cells.iter().find(|c| {
                                    c.borrow().name
                                        == (read.borrow().name.to_string()
                                            + "_spec")
                                });
                            match buf_reg {
                                Some(reg) => {
                                    reg_to_buffer
                                        .push((read.clone(), Rc::clone(&reg)));
                                    read_to_buf
                                        .push((read.clone(), Rc::clone(&reg)));
                                }
                                None => {
                                    let new_buf_reg = builder.add_primitive(
                                        read.borrow().name.to_string()
                                            + "_spec",
                                        "std_reg",
                                        &[read
                                            .borrow()
                                            .get_paramter(&ir::Id::from(
                                                WIDTH_PARAM,
                                            ))
                                            .unwrap()],
                                    );
                                    read_to_buf.push((
                                        read.clone(),
                                        new_buf_reg.clone(),
                                    ));
                                    reg_to_buffer.push((
                                        read.clone(),
                                        new_buf_reg.clone(),
                                    ));
                                }
                            }
                        }
                    }

                    builder.rename_port_writes(
                        &wrt_to_buf,
                        &mut (*spec_groups[i].borrow_mut()).assignments,
                    );
                    builder.rename_port_reads(
                        &read_to_buf,
                        &mut (*spec_groups[i].borrow_mut()).assignments,
                    );
                }

                let commit = builder.add_group("commit_spec", HashMap::new());
                let mut seen: HashSet<String> = HashSet::new();
                for (reg, reg_buf) in &reg_to_buffer {
                    if !seen.contains(&reg.borrow().name.to_string()) {
                        seen.insert(reg.borrow().name.to_string());
                        commit.borrow_mut().assignments.push(
                            builder.build_assignment(
                                reg.borrow().get("in"),
                                reg_buf.borrow().get("out"),
                                ir::Guard::True,
                            ),
                        );
                        let one_const =
                            builder.add_constant(1, 1).borrow().get("out");
                        commit.borrow_mut().assignments.push(
                            builder.build_assignment(
                                reg.borrow().get("write_en"),
                                one_const,
                                ir::Guard::True,
                            ),
                        );
                    }
                }

                let (r, _) = reg_to_buffer.first().unwrap();

                let done_port = commit.borrow().get("done");
                commit
                    .borrow_mut()
                    .assignments
                    .push(builder.build_assignment(
                        done_port,
                        r.borrow().get("done"),
                        ir::Guard::True,
                    ));

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
