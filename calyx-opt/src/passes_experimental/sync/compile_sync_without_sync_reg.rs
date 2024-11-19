use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::Guard;
use calyx_ir::Nothing;
use calyx_ir::{self as ir, GetAttributes, RRC};
use calyx_ir::{build_assignments, guard, structure};
use calyx_utils::{CalyxResult, Error};
use std::collections::HashMap;

#[derive(Default)]
/// Compiles @sync without use of std_sync_reg
/// Upon encountering @sync, it first instantiates a std_reg(1) for each thread(`bar`)
/// and a std_wire(1) for each barrier (`s`)
/// It then continuously assigns the value of (`s.in`) to 1'd1 guarded by the
/// expression that all values of `bar` for threads under the barrier are
/// set to 1'd1
/// Then it replaces the @sync control operator with
/// seq {
///     barrier;
///     clear;
/// }
/// `barrier` simply sets the value of `bar` to 1'd1 and then waits
/// for `s.out` to be up
/// `clear` resets the value of `bar` to 1'd0 for reuse of barrier
/// Using this method, each thread only incurs 3 cycles of latency overhead for
/// the barrier, and we theoretically won't have a limit for number of threads
/// under one barrier
pub struct CompileSyncWithoutSyncReg;

impl Named for CompileSyncWithoutSyncReg {
    fn name() -> &'static str {
        "compile-sync-without-sync-reg"
    }

    fn description() -> &'static str {
        "Implement barriers for statements marked with @sync attribute without std_sync_reg"
    }
}

// Data structure storing the shared `s` register and the guard accumulator
// for guarding `s.in`
#[derive(Default)]
struct BarrierMap(HashMap<u64, (RRC<ir::Cell>, Box<ir::Guard<ir::Nothing>>)>);

impl BarrierMap {
    fn get_mut(
        &mut self,
        idx: &u64,
    ) -> Option<&mut (RRC<calyx_ir::Cell>, Box<Guard<Nothing>>)> {
        self.0.get_mut(idx)
    }

    fn new() -> Self {
        BarrierMap(HashMap::new())
    }

    fn get_reg(&mut self, idx: &u64) -> &mut RRC<ir::Cell> {
        let (cell, _) = self.get_mut(idx).unwrap();
        cell
    }

    fn get_guard(&mut self, idx: &u64) -> &mut Box<ir::Guard<ir::Nothing>> {
        let (_, gd) = self.get_mut(idx).unwrap();
        gd
    }

    fn insert_shared_wire(&mut self, builder: &mut ir::Builder, idx: &u64) {
        if self.0.get(idx).is_none() {
            structure!(builder;
                let s = prim std_wire(1);
            );
            let gd = ir::Guard::True;
            self.0.insert(*idx, (s, Box::new(gd)));
        }
    }
}

// instantiates the hardware and the two groups: `bar` and `clear` for each
// barrier
fn build_barrier_group(
    builder: &mut ir::Builder,
    barrier_idx: &u64,
    barrier_reg: &mut BarrierMap,
) -> ir::Control {
    let group = builder.add_group("barrier");
    structure!(
        builder;
        let bar = prim std_reg(1);
        let z = constant(0, 1);
        let constant = constant(1, 1);
    );

    barrier_reg
        .get_guard(barrier_idx)
        .update(|g| g.and(guard!(bar["out"])));

    let s = barrier_reg.get_reg(barrier_idx);

    let assigns = build_assignments!(builder;
        bar["in"] = ? constant["out"];
        bar["write_en"] = ? constant["out"];
        group["done"] = ? s["out"];
    );
    group.borrow_mut().assignments.extend(assigns);

    let clear = builder.add_group("clear");
    let clear_assigns = build_assignments!(builder;
        bar["in"] = ? z["out"];
        bar["write_en"] = ? constant["out"];
        clear["done"] = ? bar["done"];);
    clear.borrow_mut().assignments.extend(clear_assigns);

    let stmts = vec![ir::Control::enable(group), ir::Control::enable(clear)];

    ir::Control::seq(stmts)
}

// produces error if `invoke` or `enable` are marked with @sync
fn produce_err(con: &ir::Control) -> CalyxResult<()> {
    match con {
        ir::Control::Enable(e) => {
            if con.get_attributes().get(ir::NumAttr::Sync).is_some() {
                return Err(Error::malformed_control(
                    "Enable or Invoke controls cannot be marked with @sync"
                        .to_string(),
                )
                .with_pos(e.get_attributes()));
            }
            Ok(())
        }
        ir::Control::Invoke(i) => {
            if con.get_attributes().get(ir::NumAttr::Sync).is_some() {
                return Err(Error::malformed_control(
                    "Enable or Invoke controls cannot be marked with @sync"
                        .to_string(),
                )
                .with_pos(&i.attributes));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

// recursively looks for the `@sync` control operator and then replaces them with
// the corresponding `seq` block
fn insert_barrier(
    builder: &mut ir::Builder,
    con: &mut ir::Control,
    barrier_reg: &mut BarrierMap,
    barrier_con: &mut HashMap<u64, ir::Control>,
) -> CalyxResult<()> {
    match con {
        ir::Control::Empty(_) => {
            if let Some(ref n) = con.get_attributes().get(ir::NumAttr::Sync) {
                barrier_reg.insert_shared_wire(builder, n);
                let con_ref = barrier_con.entry(*n).or_insert_with(|| {
                    build_barrier_group(builder, n, barrier_reg)
                });
                std::mem::swap(con, &mut ir::Cloner::control(con_ref));
            }
            Ok(())
        }
        ir::Control::Seq(seq) => {
            for s in seq.stmts.iter_mut() {
                insert_barrier(builder, s, barrier_reg, barrier_con)?;
            }
            Ok(())
        }
        ir::Control::If(i) => {
            insert_barrier(builder, &mut i.tbranch, barrier_reg, barrier_con)?;
            insert_barrier(builder, &mut i.fbranch, barrier_reg, barrier_con)?;
            Ok(())
        }
        ir::Control::While(w) => {
            insert_barrier(builder, &mut w.body, barrier_reg, barrier_con)?;
            Ok(())
        }
        ir::Control::Enable(_) | ir::Control::Invoke(_) => {
            produce_err(con)?;
            Ok(())
        }
        _ => Ok(()),
    }
}
impl Visitor for CompileSyncWithoutSyncReg {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let mut barrier_reg: BarrierMap = BarrierMap::new();
        for stmt in s.stmts.iter_mut() {
            let mut barrier_con: HashMap<u64, ir::Control> = HashMap::new();
            insert_barrier(
                &mut builder,
                stmt,
                &mut barrier_reg,
                &mut barrier_con,
            )?;
        }

        // add continuous assignments for value of `s`
        for (_, (wire, g_box)) in barrier_reg.0 {
            structure!( builder;
                let constant = constant(1,1);
            );
            let g = *g_box;
            let cont_assigns = build_assignments!(builder;
                wire["in"] = g ? constant["out"];
            );
            builder
                .component
                .continuous_assignments
                .extend(cont_assigns);
        }
        Ok(Action::Continue)
    }
}
