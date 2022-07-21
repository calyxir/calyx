use std::collections::HashMap;

use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::RRC;
use crate::ir::{self, GetAttributes};
use linked_hash_map::LinkedHashMap;

#[derive(Default)]
/// 1. loop through all control statements under "par" block to find # barriers
///    needed and # members of each barrier
/// 2. add all cells and groups needed
/// 3. loop through all control statements, find the statements with @sync
///    attribute and replace them with
///     seq {
///       <stmt>;
///       incr_barrier_0_*;
///       write_barrier_0_*;
///       wait_*;
///       restore_*;
///     }
///    or
///     seq {
///       <stmt>;
///       incr_barrier_*_*;
///       write_barrier_*_*;
///       wait_*;
///       wait_restore_*;
///     }

pub struct CompileSync;

impl Named for CompileSync {
    fn name() -> &'static str {
        "compile-sync"
    }

    fn description() -> &'static str {
        "Implement barriers for statements marked with @sync attribute"
    }
}

fn count_barriers(s: &ir::Control, map: &mut LinkedHashMap<u64, u64>) {
    match s {
        ir::Control::Seq(seq) => {
            for stmt in seq.stmts.iter() {
                if let Some(n) = stmt.get_attributes().unwrap().get("sync") {
                    map.entry(*n).and_modify(|count| *count += 1).or_insert(1);
                }
            }
        }
        ir::Control::While(w) => {
            count_barriers(&w.body, map);
        }
        ir::Control::If(i) => {
            count_barriers(&i.tbranch, map);
            count_barriers(&i.fbranch, map);
        }
        _ => {}
    }
}

fn fmt_shared_name(comp: &'static str, idx: &u64) -> ir::Id {
    format!("{}_{}", comp, idx).into()
}

fn fmt_ind_name(
    comp: &'static str,
    idx_barrier: &u64,
    idx_member: &u64,
) -> ir::Id {
    format!("{}_{}_{}", comp, idx_barrier, idx_member).into()
}

fn add_shared_primitive(
    builder: &mut ir::Builder,
    prefix: &'static str,
    barrier_idx: &u64,
    primitive: &str,
    parameters: &[u64],
    map: &mut HashMap<ir::Id, ir::Id>,
) -> RRC<ir::Cell> {
    let prim = builder.add_primitive(
        fmt_shared_name(prefix, barrier_idx),
        primitive,
        parameters,
    );
    map.insert(
        fmt_shared_name(prefix, barrier_idx),
        prim.borrow().name().clone(),
    );
    prim
}

fn add_ind_primitive(
    builder: &mut ir::Builder,
    prefix: &'static str,
    barrier_idx: &u64,
    member_idx: &u64,
    primitive: &str,
    parameters: &[u64],
    map: &mut HashMap<ir::Id, ir::Id>,
) -> RRC<ir::Cell> {
    let prim = builder.add_primitive(
        fmt_ind_name(prefix, barrier_idx, member_idx),
        primitive,
        parameters,
    );
    map.insert(
        fmt_ind_name(prefix, barrier_idx, member_idx),
        prim.borrow().name().clone(),
    );
    prim
}

fn add_shared_group(
    builder: &mut ir::Builder,
    prefix: &'static str,
    barrier_idx: &u64,
    map: &mut HashMap<ir::Id, ir::Id>,
) -> RRC<ir::Group> {
    let group = builder.add_group(fmt_shared_name(prefix, barrier_idx));
    map.insert(
        fmt_shared_name(prefix, barrier_idx),
        group.borrow().name().clone(),
    );
    group
}

fn add_ind_group(
    builder: &mut ir::Builder,
    prefix: &'static str,
    barrier_idx: &u64,
    member_idx: &u64,
    map: &mut HashMap<ir::Id, ir::Id>,
) -> RRC<ir::Group> {
    let group =
        builder.add_group(fmt_ind_name(prefix, barrier_idx, member_idx));
    map.insert(
        fmt_ind_name(prefix, barrier_idx, member_idx),
        group.borrow().name().clone(),
    );
    group
}

fn add_assignment_to_group(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    dst: RRC<ir::Port>,
    src: RRC<ir::Port>,
    guard: ir::Guard,
) {
    let asmt = builder.build_assignment(dst, src, guard);
    group.borrow_mut().assignments.push(asmt);
}

fn find_port(
    comp: &mut ir::Component,
    cell_name: &ir::Id,
    port_name: &str,
    map: &HashMap<ir::Id, ir::Id>,
) -> RRC<ir::Port> {
    comp.find_cell(map.get(cell_name).unwrap())
        .unwrap()
        .borrow()
        .get(port_name)
}

fn find_group(
    comp: &mut ir::Component,
    group_name: &ir::Id,
    map: &HashMap<ir::Id, ir::Id>,
) -> RRC<ir::Group> {
    comp.find_group(map.get(group_name).unwrap()).unwrap()
}

fn build_incr_barrier(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier_idx: &u64,
    member_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) {
    // barrier_*.read_en_0 = 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("read_en_{}", member_idx),
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    //incr_*_*.left = barrier_*.read_done_*?barrier_*.out_*;
    let dst = find_port(
        builder.component,
        &fmt_ind_name("incr", barrier_idx, member_idx),
        "left",
        map,
    );
    let src = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("out_{}", member_idx),
        map,
    );
    let guard = ir::Guard::Port(find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("read_done_{}", member_idx),
        map,
    ));
    add_assignment_to_group(builder, group, dst, src, guard);
    // incr_*_*.right = barrier_*.read_done_*?32'd1;
    let dst = find_port(
        builder.component,
        &fmt_ind_name("incr", barrier_idx, member_idx),
        "right",
        map,
    );
    let src = builder.add_constant(1, 32).borrow().get("out");
    let guard = ir::Guard::Port(find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("read_done_{}", member_idx),
        map,
    ));
    add_assignment_to_group(builder, group, dst, src, guard);
    // save_*_*.in = barrier_*.read_done_*? incr_1.out;
    let dst = find_port(
        builder.component,
        &fmt_ind_name("save", barrier_idx, member_idx),
        "in",
        map,
    );
    let src = find_port(
        builder.component,
        &fmt_ind_name("incr", barrier_idx, member_idx),
        "out",
        map,
    );
    let guard = ir::Guard::Port(find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("read_done_{}", member_idx),
        map,
    ));
    add_assignment_to_group(builder, group, dst, src, guard);
    // save_*_*.write_en = barrier_*.read_done_*;
    let dst = find_port(
        builder.component,
        &fmt_ind_name("save", barrier_idx, member_idx),
        "write_en",
        map,
    );
    let src = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("read_done_{}", member_idx),
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    // incr_barrier_*_*[done] = save_0_1.done;
    let dst = group.borrow().get("done");
    let src = find_port(
        builder.component,
        &fmt_ind_name("save", barrier_idx, member_idx),
        "done",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
}

fn build_write_barrier(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier_idx: &u64,
    member_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) {
    // barrier_*.write_en_* = 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("write_en_{}", member_idx),
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    // barrier_*.in_* = save_*_*.out;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("in_{}", member_idx),
        map,
    );
    let src = find_port(
        builder.component,
        &fmt_ind_name("save", barrier_idx, member_idx),
        "out",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    // write_barrier_*_*[done] = barrier_*.write_done_*;
    let dst = group.borrow().get("done");
    let src = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        &format!("write_done_{}", member_idx),
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
}

fn build_wait(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier_idx: &u64,
    member_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) {
    // wait_reg_*.in = eq_*.out;
    let dst = find_port(
        builder.component,
        &fmt_ind_name("wait_reg", barrier_idx, member_idx),
        "in",
        map,
    );
    let src = find_port(
        builder.component,
        &fmt_shared_name("eq", barrier_idx),
        "out",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    // wait_reg_*.write_en = eq_*.out? 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_ind_name("wait_reg", barrier_idx, member_idx),
        "write_en",
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::port(find_port(
        builder.component,
        &fmt_shared_name("eq", barrier_idx),
        "out",
        map,
    ));
    add_assignment_to_group(builder, group, dst, src, guard);
    // wait_*[done] = wait_reg_*.done;
    let dst = group.borrow().get("done");
    let src = find_port(
        builder.component,
        &fmt_ind_name("wait_reg", barrier_idx, member_idx),
        "done",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
}

fn build_clear_barrier(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) {
    // barrier_*.read_en_0 = 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        "read_en_0",
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    //clear_barrier_*[done] = barrier_1.read_done_0;
    let dst = group.borrow().get("done");
    let src = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        "read_done_0",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
}

fn build_restore(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) {
    // barrier_*.write_en_0 = 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        "write_en_0",
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    // barrier_*.in_0 = 32'd0;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        "in_0",
        map,
    );
    let src = builder.add_constant(0, 32).borrow().get("out");
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
    // restore_*[done] = barrier_*.write_done_0;
    let dst = group.borrow().get("done");
    let src = find_port(
        builder.component,
        &fmt_shared_name("barrier", barrier_idx),
        "write_done_0",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
}

fn build_wait_restore(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) {
    // wait_restore_reg_*.in = !eq_*.out? 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("wait_restore_reg", barrier_idx),
        "in",
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::Not(Box::new(ir::Guard::Port(find_port(
        builder.component,
        &fmt_shared_name("eq", barrier_idx),
        "out",
        map,
    ))));
    add_assignment_to_group(builder, group, dst, src, guard);
    // wait_restore_reg_*.write_en = !eq_*.out? 1'd1;
    let dst = find_port(
        builder.component,
        &fmt_shared_name("wait_restore_reg", barrier_idx),
        "write_en",
        map,
    );
    let src = builder.add_constant(1, 1).borrow().get("out");
    let guard = ir::Guard::Not(Box::new(ir::Guard::Port(find_port(
        builder.component,
        &fmt_shared_name("eq", barrier_idx),
        "out",
        map,
    ))));
    add_assignment_to_group(builder, group, dst, src, guard);
    //wait_restore_*[done] = wait_restore_reg_*.out;
    let dst = group.borrow().get("done");
    let src = find_port(
        builder.component,
        &fmt_shared_name("wait_restore_reg", barrier_idx),
        "done",
        map,
    );
    let guard = ir::Guard::True;
    add_assignment_to_group(builder, group, dst, src, guard);
}

fn build_member_0(
    builder: &mut ir::Builder,
    original: &ir::Control,
    barrier_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) -> ir::Control {
    let mut stmts: Vec<ir::Control> = Vec::new();
    let mut copy = ir::Control::clone(original);

    copy.get_mut_attributes().unwrap().remove("sync");

    stmts.push(copy);
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_ind_name("incr_barrier", barrier_idx, &0),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_ind_name("write_barrier", barrier_idx, &0),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_ind_name("wait", barrier_idx, &0),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_shared_name("clear_barrier", barrier_idx),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_shared_name("restore", barrier_idx),
        map,
    )));
    ir::Control::seq(stmts)
}

fn build_member(
    builder: &mut ir::Builder,
    original: &ir::Control,
    barrier_idx: &u64,
    member_idx: &u64,
    map: &HashMap<ir::Id, ir::Id>,
) -> ir::Control {
    let mut stmts: Vec<ir::Control> = Vec::new();
    let mut copy = ir::Control::clone(original);

    copy.get_mut_attributes().unwrap().remove("sync");

    stmts.push(copy);
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_ind_name("incr_barrier", barrier_idx, member_idx),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_ind_name("write_barrier", barrier_idx, member_idx),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_ind_name("wait", barrier_idx, member_idx),
        map,
    )));
    stmts.push(ir::Control::enable(find_group(
        builder.component,
        &fmt_shared_name("wait_restore", barrier_idx),
        map,
    )));
    ir::Control::seq(stmts)
}

fn insert_control(
    s: &mut ir::Control,
    builder: &mut ir::Builder,
    barrier_count: &mut LinkedHashMap<u64, u64>,
    group_name_map: &mut HashMap<ir::Id, ir::Id>,
) {
    match s {
        ir::Control::Seq(seq) => {
            let mut stmts_new: Vec<ir::Control> = Vec::new();
            for stmt in seq.stmts.iter_mut() {
                if let Some(n) = stmt.get_attributes().unwrap().get("sync") {
                    barrier_count
                        .entry(*n)
                        .and_modify(|count| *count += 1)
                        .or_insert(0);
                    if barrier_count.get(n).unwrap() == &0 {
                        stmts_new.push(build_member_0(
                            builder,
                            stmt,
                            n,
                            group_name_map,
                        ));
                    } else {
                        stmts_new.push(build_member(
                            builder,
                            stmt,
                            n,
                            barrier_count.get(n).unwrap(),
                            group_name_map,
                        ));
                    }
                } else {
                    stmts_new.push(ir::Control::clone(stmt));
                }
            }
            seq.stmts = stmts_new;
        }
        ir::Control::If(i) => {
            insert_control(
                &mut i.tbranch,
                builder,
                barrier_count,
                group_name_map,
            );
            insert_control(
                &mut i.fbranch,
                builder,
                barrier_count,
                group_name_map,
            );
        }

        ir::Control::While(w) => {
            insert_control(&mut w.body, builder, barrier_count, group_name_map);
        }
        _ => {}
    }
}

impl Visitor for CompileSync {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // count # barriers and # members for each barrier
        let mut barriers: LinkedHashMap<u64, u64> = LinkedHashMap::new();
        let mut barrier_count: LinkedHashMap<u64, u64> = LinkedHashMap::new();
        let mut cell_name_map: HashMap<ir::Id, ir::Id> = HashMap::new();
        let mut group_name_map: HashMap<ir::Id, ir::Id> = HashMap::new();
        for stmt in s.stmts.iter() {
            count_barriers(stmt, &mut barriers);
        }

        if barriers.is_empty() {
            return Ok(Action::Continue);
        }

        let mut builder = ir::Builder::new(comp, sigs);

        // for each barrier, add cells needed for implementation
        // for each barrier, we need
        // 1. 1 std_sync_reg(32)
        // 2. 1 std_eq(32)
        // 3. 3 std_reg(1)
        // 4. n std_reg(32)
        // 5. n std_add(32)
        for (idx, n_members) in barriers.iter() {
            let v_32: Vec<u64> = vec![32];
            let v_1: Vec<u64> = vec![1];
            add_shared_primitive(
                &mut builder,
                "barrier",
                idx,
                "std_sync_reg",
                &v_32,
                &mut cell_name_map,
            );
            add_shared_primitive(
                &mut builder,
                "eq",
                idx,
                "std_eq",
                &v_32,
                &mut cell_name_map,
            );
            add_shared_primitive(
                &mut builder,
                "wait_restore_reg",
                idx,
                "std_reg",
                &v_1,
                &mut cell_name_map,
            );
            for n_member in 0..*n_members {
                add_ind_primitive(
                    &mut builder,
                    "save",
                    idx,
                    &n_member,
                    "std_reg",
                    &v_32,
                    &mut cell_name_map,
                );
                add_ind_primitive(
                    &mut builder,
                    "wait_reg",
                    idx,
                    &n_member,
                    "std_reg",
                    &v_1,
                    &mut cell_name_map,
                );
                add_ind_primitive(
                    &mut builder,
                    "incr",
                    idx,
                    &n_member,
                    "std_add",
                    &v_32,
                    &mut cell_name_map,
                );
            }
        }

        // for each barrier, add groups needed for implementation
        // for each barrier, we need
        // 1. n incr_barrier
        // 2. n write_barrier
        // 3. 1 wait
        // 4. 1 restore
        // 5. 1 wait_restore
        for (idx, n_members) in barriers.iter() {
            for n_member in 0..*n_members {
                let incr_barrier = add_ind_group(
                    &mut builder,
                    "incr_barrier",
                    idx,
                    &n_member,
                    &mut group_name_map,
                );
                let write_barrier = add_ind_group(
                    &mut builder,
                    "write_barrier",
                    idx,
                    &n_member,
                    &mut group_name_map,
                );
                let wait = add_ind_group(
                    &mut builder,
                    "wait",
                    idx,
                    &n_member,
                    &mut group_name_map,
                );
                build_wait(&mut builder, &wait, idx, &n_member, &cell_name_map);
                build_incr_barrier(
                    &mut builder,
                    &incr_barrier,
                    idx,
                    &n_member,
                    &cell_name_map,
                );
                build_write_barrier(
                    &mut builder,
                    &write_barrier,
                    idx,
                    &n_member,
                    &cell_name_map,
                );
            }
            let restore = add_shared_group(
                &mut builder,
                "restore",
                idx,
                &mut group_name_map,
            );
            let clear_barrier = add_shared_group(
                &mut builder,
                "clear_barrier",
                idx,
                &mut group_name_map,
            );
            let wait_restore = add_shared_group(
                &mut builder,
                "wait_restore",
                idx,
                &mut group_name_map,
            );
            build_restore(&mut builder, &restore, idx, &cell_name_map);
            build_wait_restore(
                &mut builder,
                &wait_restore,
                idx,
                &cell_name_map,
            );
            build_clear_barrier(
                &mut builder,
                &clear_barrier,
                idx,
                &cell_name_map,
            );

            // add continuous assignments
            // eq_*.left = barrier_*.peek;
            let src = find_port(
                builder.component,
                &fmt_shared_name("eq", idx),
                "left",
                &cell_name_map,
            );
            let dst = find_port(
                builder.component,
                &fmt_shared_name("barrier", idx),
                "peek",
                &cell_name_map,
            );
            let guard = ir::Guard::True;
            builder
                .component
                .continuous_assignments
                .push(builder.build_assignment(src, dst, guard));
            // eq_*.right = 32'd* (number of members);
            let src = find_port(
                builder.component,
                &fmt_shared_name("eq", idx),
                "right",
                &cell_name_map,
            );
            let dst = builder.add_constant(*n_members, 32).borrow().get("out");
            let guard = ir::Guard::True;
            builder
                .component
                .continuous_assignments
                .push(builder.build_assignment(src, dst, guard));
        }

        // replace @sync <stmt> with
        // seq {
        // <stmt>;
        // incr_barrier_*_*;
        // write_barrier_*_*;
        // wait_*;
        // restore_*;
        // } or
        // seq {
        // <stmt>;
        // incr_barrier_*_*;
        // write_barrier_*_*;
        // wait_*;
        // wait_restore_*;
        // }
        for stmt in s.stmts.iter_mut() {
            insert_control(
                stmt,
                &mut builder,
                &mut barrier_count,
                &mut group_name_map,
            );
        }

        let mut init_barriers: Vec<ir::Control> = Vec::new();
        for (idx, _) in barrier_count.iter() {
            init_barriers.push(ir::Control::enable(find_group(
                builder.component,
                &fmt_shared_name("restore", idx),
                &group_name_map,
            )));
        }

        let mut changed_sequence: Vec<ir::Control> =
            vec![ir::Control::par(init_barriers)];
        let mut copied_par_stmts: Vec<ir::Control> = Vec::new();
        for con in s.stmts.drain(..) {
            copied_par_stmts.push(con);
        }
        changed_sequence.push(ir::Control::par(copied_par_stmts));

        Ok(Action::change(ir::Control::seq(changed_sequence)))
    }
}
