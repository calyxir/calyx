use std::collections::HashMap;
use std::rc::Rc;

use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::RRC;
use crate::ir::{self, GetAttributes};
use crate::{build_assignments, guard, structure};
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

type BarrierMap = LinkedHashMap<u64, (Vec<RRC<ir::Cell>>, Vec<RRC<ir::Group>>)>;

impl Named for CompileSync {
    fn name() -> &'static str {
        "compile-sync"
    }

    fn description() -> &'static str {
        "Implement barriers for statements marked with @sync attribute"
    }
}

fn count_barriers(
    builder: &mut ir::Builder,
    s: &mut ir::Control,
    map: &mut BarrierMap,
    count: &mut HashMap<u64, u64>,
) {
    match s {
        ir::Control::Seq(seq) => {
            let mut stmts_new: Vec<ir::Control> = Vec::new();
            for stmt in seq.stmts.drain(..) {
                if let Some(n) = stmt.get_attributes().unwrap().get("sync") {
                    if map.get(n).is_none() {
                        add_shared_structure(builder, n, map);
                    }
                    let (cells, groups) = map.get(n).unwrap();
                    let barrier = Rc::clone(&cells[0]);
                    let eq = Rc::clone(&cells[1]);
                    let wait_restore = Rc::clone(&groups[0]);
                    let restore = Rc::clone(&groups[1]);
                    let clear_barrier = Rc::clone(&groups[2]);
                    let n_new = *(count.entry(*n).or_insert(0)) + 1;
                    count.insert(*n, n_new);
                    structure!(builder;
                            let save = prim std_reg(32););
                    let incr_barrier = builder.add_group("incr_barrier");
                    build_incr_barrier(
                        builder,
                        &incr_barrier,
                        &barrier,
                        &save,
                        &(&n_new - 1),
                    );
                    let write_barrier = builder.add_group("write_barrier");
                    build_write_barrier(
                        builder,
                        &write_barrier,
                        &barrier,
                        &save,
                        &(&n_new - 1),
                    );
                    let wait = builder.add_group("wait");
                    build_wait(builder, &wait, &eq);
                    if n_new == 1 {
                        stmts_new.push(build_member(
                            &stmt,
                            incr_barrier,
                            write_barrier,
                            wait,
                            wait_restore,
                        ));
                    } else {
                        stmts_new.push(build_member_0(
                            &stmt,
                            incr_barrier,
                            write_barrier,
                            wait,
                            clear_barrier,
                            restore,
                        ));
                    }
                } else {
                    stmts_new.push(stmt);
                }
            }
            seq.stmts = stmts_new;
        }
        ir::Control::While(w) => {
            count_barriers(builder, &mut w.body, map, count);
        }
        ir::Control::If(i) => {
            count_barriers(builder, &mut i.tbranch, map, count);
            count_barriers(builder, &mut i.fbranch, map, count);
        }
        _ => {}
    }
}

fn build_incr_barrier(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier: &RRC<ir::Cell>,
    save: &RRC<ir::Cell>,
    member_idx: &u64,
) {
    structure!(builder;
        let incr = prim std_add(32);
        let cst_1 = constant(1, 1);
        let cst_2 = constant(1, 32););
    let read_done_guard = guard!(barrier[format!("read_done_{member_idx}")]);
    let mut assigns = build_assignments!(builder;
        // barrier_*.read_en_0 = 1'd1;
        barrier[format!("read_en_{member_idx}")] = ?cst_1["out"];
        //incr_*_*.left = barrier_*.read_done_*?barrier_*.out_*;
        incr["left"] = read_done_guard? barrier[format!("out_{member_idx}")];
        // incr_*_*.right = barrier_*.read_done_*?32'd1;
        incr["right"] = read_done_guard? cst_2["out"];
        // save_*_*.in = barrier_*.read_done_*? incr_1.out;
        save["in"] = read_done_guard? incr["out"];
        // save_*_*.write_en = barrier_*.read_done_*;
        save["write_en"] = ? barrier[format!("read_done_{member_idx}")];
        // incr_barrier_*_*[done] = save_*_*.done;
        group["done"] = ?save["done"];
    );

    group.borrow_mut().assignments.append(&mut assigns);
}

fn build_write_barrier(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier: &RRC<ir::Cell>,
    save: &RRC<ir::Cell>,
    member_idx: &u64,
) {
    structure!(builder;
    let cst_1 = constant(1, 1););
    let mut assigns = build_assignments!(builder;
        // barrier_*.write_en_* = 1'd1;
        barrier[format!("write_en_{member_idx}")] = ?cst_1["out"];
        // barrier_*.in_* = save_*_*.out;
        barrier[format!("in_{member_idx}")] = ?save["out"];
        // write_barrier_*_*[done] = barrier_*.write_done_*;
        group["done"] = ?barrier[format!("write_done_{member_idx}")];
    );
    group.borrow_mut().assignments.append(&mut assigns);
}

fn build_wait(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    eq: &RRC<ir::Cell>,
) {
    structure!(builder;
    let wait_reg = prim std_reg(1);
    let cst_1 = constant(1, 1););
    let eq_guard = guard!(eq["out"]);
    let mut assigns = build_assignments!(builder;
        // wait_reg_*.in = eq_*.out;
        wait_reg["in"] = ?eq["out"];
        // wait_reg_*.write_en = eq_*.out? 1'd1;
        wait_reg["write_en"] = eq_guard? cst_1["out"];
        // wait_*[done] = wait_reg_*.done;
        group["done"] = ?wait_reg["done"];);
    group.borrow_mut().assignments.append(&mut assigns);
}

fn build_clear_barrier(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier: &RRC<ir::Cell>,
) {
    structure!(builder;
    let cst_1 = constant(1, 1););
    let mut assigns = build_assignments!(builder;
    // barrier_*.read_en_0 = 1'd1;
    barrier["read_en_0"] = ?cst_1["out"];
    //clear_barrier_*[done] = barrier_1.read_done_0;
    group["done"] = ?barrier["read_done_0"];
    );
    group.borrow_mut().assignments.append(&mut assigns);
}

fn build_restore(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    barrier: &RRC<ir::Cell>,
) {
    structure!(builder;
    let cst_1 = constant(1,1);
    let cst_2 = constant(0, 32););
    let mut assigns = build_assignments!(builder;
        // barrier_*.write_en_0 = 1'd1;
        barrier["write_en_0"] = ?cst_1["out"];
        // barrier_*.in_0 = 32'd0;
        barrier["in_0"] = ?cst_2["out"];
        // restore_*[done] = barrier_*.write_done_0;
        group["done"] = ?barrier["write_done_0"];
    );
    group.borrow_mut().assignments.append(&mut assigns);
}

fn build_wait_restore(
    builder: &mut ir::Builder,
    group: &RRC<ir::Group>,
    eq: &RRC<ir::Cell>,
) {
    structure!(builder;
    let wait_restore_reg = prim std_reg(1);
    let cst_1 = constant(1, 1););
    let eq_guard = !guard!(eq["out"]);
    let mut assigns = build_assignments!(builder;
    // wait_restore_reg_*.in = !eq_*.out? 1'd1;
    wait_restore_reg["in"] = eq_guard? cst_1["out"];
    // wait_restore_reg_*.write_en = !eq_*.out? 1'd1;
    wait_restore_reg["write_en"] = eq_guard? cst_1["out"];
    //wait_restore_*[done] = wait_restore_reg_*.done;
    group["done"] = ?wait_restore_reg["done"];
    );
    group.borrow_mut().assignments.append(&mut assigns);
}

fn build_member_0(
    original: &ir::Control,
    incr_barrier: RRC<ir::Group>,
    write_barrier: RRC<ir::Group>,
    wait: RRC<ir::Group>,
    clear_barrier: RRC<ir::Group>,
    restore: RRC<ir::Group>,
) -> ir::Control {
    let mut stmts: Vec<ir::Control> = Vec::new();
    let mut copy = ir::Control::clone(original);

    copy.get_mut_attributes().unwrap().remove("sync");

    stmts.push(copy);
    stmts.push(ir::Control::enable(incr_barrier));
    stmts.push(ir::Control::enable(write_barrier));
    stmts.push(ir::Control::enable(wait));
    stmts.push(ir::Control::enable(clear_barrier));
    stmts.push(ir::Control::enable(restore));
    ir::Control::seq(stmts)
}

fn build_member(
    original: &ir::Control,
    incr_barrier: RRC<ir::Group>,
    write_barrier: RRC<ir::Group>,
    wait: RRC<ir::Group>,
    wait_restore: RRC<ir::Group>,
) -> ir::Control {
    let mut stmts: Vec<ir::Control> = Vec::new();
    let mut copy = ir::Control::clone(original);

    copy.get_mut_attributes().unwrap().remove("sync");

    stmts.push(copy);
    stmts.push(ir::Control::enable(incr_barrier));
    stmts.push(ir::Control::enable(write_barrier));
    stmts.push(ir::Control::enable(wait));
    stmts.push(ir::Control::enable(wait_restore));
    ir::Control::seq(stmts)
}

fn add_shared_structure(
    builder: &mut ir::Builder,
    barrier_idx: &u64,
    map: &mut BarrierMap,
) {
    structure!(builder;
            let barrier = prim std_sync_reg(32);
            let eq = prim std_eq(32);
    );
    let wait_restore = builder.add_group("wait_restore");
    let restore = builder.add_group("restore");
    let clear_barrier = builder.add_group("clear_barrier");
    build_restore(builder, &restore, &barrier);
    build_wait_restore(builder, &wait_restore, &eq);
    build_clear_barrier(builder, &clear_barrier, &barrier);
    let shared_cells: Vec<RRC<ir::Cell>> = vec![barrier, eq];
    let shared_groups: Vec<RRC<ir::Group>> =
        vec![wait_restore, restore, clear_barrier];
    let info = (shared_cells, shared_groups);
    map.insert(*barrier_idx, info);
}

impl Visitor for CompileSync {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut barriers: BarrierMap = LinkedHashMap::new();
        let mut builder = ir::Builder::new(comp, sigs);
        let mut barrier_count: HashMap<u64, u64> = HashMap::new();
        for stmt in s.stmts.iter_mut() {
            count_barriers(
                &mut builder,
                stmt,
                &mut barriers,
                &mut barrier_count,
            );
        }

        if barriers.is_empty() {
            return Ok(Action::Continue);
        }

        let mut init_barriers: Vec<ir::Control> = Vec::new();
        for (n, (cells, groups)) in barriers.iter() {
            let barrier = Rc::clone(&cells[0]);
            let eq = Rc::clone(&cells[1]);
            let restore = Rc::clone(&groups[1]);
            let n_members = barrier_count.get(n).unwrap();
            structure!(builder;
                let num_members = constant(*n_members, 32);
            );
            // add continuous assignments
            let mut assigns = build_assignments!(builder;
            // eq_*.left = barrier_*.peek;
            eq["left"] = ?barrier["peek"];
            // eq_*.right = 32'd* (number of members);
            eq["right"] = ?num_members["out"];
            );
            builder
                .component
                .continuous_assignments
                .append(&mut assigns);

            init_barriers.push(ir::Control::enable(restore));
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
