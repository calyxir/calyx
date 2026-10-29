use crate::analysis::bank_report::{
    AccessKind, AccessSite, DepKind, MemoryDependence,
};
use crate::analysis::memory_access::{
    MemKind, MemoryAccess, accesses_in_assignments, memory_kinds,
    static_accesses_in_assignments,
};
use calyx_ir as ir;
use std::collections::HashMap;

fn classify(from: AccessKind, to: AccessKind) -> Option<DepKind> {
    match (from, to) {
        (AccessKind::Write, AccessKind::Read) => Some(DepKind::Raw),
        (AccessKind::Read, AccessKind::Write) => Some(DepKind::War),
        (AccessKind::Write, AccessKind::Write) => Some(DepKind::Waw),
        (AccessKind::Read, AccessKind::Read) => None,
    }
}

fn gather(
    control: &ir::Control,
    mems: &HashMap<ir::Id, MemKind>,
    out: &mut Vec<MemoryAccess>,
) {
    match control {
        ir::Control::Empty(_)
        | ir::Control::FSMEnable(_)
        | ir::Control::Invoke(_) => {}
        ir::Control::Enable(ir::Enable { group, .. }) => {
            let group = group.borrow();
            out.extend(accesses_in_assignments(
                group.name(),
                &group.assignments,
                mems,
            ));
        }
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            for s in stmts {
                gather(s, mems, out);
            }
        }
        ir::Control::If(ir::If {
            cond,
            tbranch,
            fbranch,
            ..
        }) => {
            if let Some(cg) = cond {
                let cg = cg.borrow();
                out.extend(accesses_in_assignments(
                    cg.name(),
                    &cg.assignments,
                    mems,
                ));
            }
            gather(tbranch, mems, out);
            gather(fbranch, mems, out);
        }
        ir::Control::While(ir::While { cond, body, .. }) => {
            if let Some(cg) = cond {
                let cg = cg.borrow();
                out.extend(accesses_in_assignments(
                    cg.name(),
                    &cg.assignments,
                    mems,
                ));
            }
            gather(body, mems, out);
        }
        ir::Control::Repeat(ir::Repeat { body, .. }) => gather(body, mems, out),
        ir::Control::Static(sc) => gather_static(sc, mems, out),
    }
}

fn gather_static(
    control: &ir::StaticControl,
    mems: &HashMap<ir::Id, MemKind>,
    out: &mut Vec<MemoryAccess>,
) {
    match control {
        ir::StaticControl::Empty(_) | ir::StaticControl::Invoke(_) => {}
        ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
            let group = group.borrow();
            out.extend(static_accesses_in_assignments(
                group.name(),
                &group.assignments,
                mems,
            ));
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
        | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            for s in stmts {
                gather_static(s, mems, out);
            }
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            gather_static(tbranch, mems, out);
            gather_static(fbranch, mems, out);
        }
        ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
            gather_static(body, mems, out)
        }
    }
}

fn emit_carried(body: &[MemoryAccess], out: &mut Vec<MemoryDependence>) {
    for from in body {
        for to in body {
            if from.memory != to.memory {
                continue;
            }
            if let Some(kind) = classify(from.kind, to.kind) {
                out.push(MemoryDependence {
                    memory: from.memory,
                    from: AccessSite::new(from.site, from.kind),
                    to: AccessSite::new(to.site, to.kind),
                    kind,
                });
            }
        }
    }
}

fn walk(
    control: &ir::Control,
    mems: &HashMap<ir::Id, MemKind>,
    out: &mut Vec<MemoryDependence>,
) {
    match control {
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            for s in stmts {
                walk(s, mems, out);
            }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            walk(tbranch, mems, out);
            walk(fbranch, mems, out);
        }
        ir::Control::While(ir::While { body, .. }) => {
            let mut body_accesses = Vec::new();
            gather(body, mems, &mut body_accesses);
            emit_carried(&body_accesses, out);
            walk(body, mems, out);
        }
        ir::Control::Repeat(ir::Repeat {
            body, num_repeats, ..
        }) => {
            if *num_repeats >= 2 {
                let mut body_accesses = Vec::new();
                gather(body, mems, &mut body_accesses);
                emit_carried(&body_accesses, out);
            }
            walk(body, mems, out);
        }
        ir::Control::Static(sc) => walk_static(sc, mems, out),
        _ => {}
    }
}

fn walk_static(
    control: &ir::StaticControl,
    mems: &HashMap<ir::Id, MemKind>,
    out: &mut Vec<MemoryDependence>,
) {
    match control {
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
        | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            for s in stmts {
                walk_static(s, mems, out);
            }
        }
        ir::StaticControl::If(ir::StaticIf {
            tbranch, fbranch, ..
        }) => {
            walk_static(tbranch, mems, out);
            walk_static(fbranch, mems, out);
        }
        ir::StaticControl::Repeat(ir::StaticRepeat {
            body,
            num_repeats,
            ..
        }) => {
            if *num_repeats >= 2 {
                let mut body_accesses = Vec::new();
                gather_static(body, mems, &mut body_accesses);
                emit_carried(&body_accesses, out);
            }
            walk_static(body, mems, out);
        }
        _ => {}
    }
}

pub fn loop_dependences(comp: &ir::Component) -> Vec<MemoryDependence> {
    let mems = memory_kinds(comp);
    let mut out = Vec::new();
    let control = comp.control.borrow();
    walk(&control, &mems, &mut out);
    out
}
