use crate::analysis::bank_report::AccessKind;
use calyx_ir as ir;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemKind {
    Comb,
    Seq(u64),
}

pub fn classify_memory(cell: &ir::Cell) -> Option<MemKind> {
    if let ir::CellType::Primitive { name, latency, .. } = &cell.prototype {
        let n = name.id.as_str();
        if n.starts_with("comb_mem_d") {
            return Some(MemKind::Comb);
        }
        if n.starts_with("seq_mem_d") {
            let lat = latency.map(|l| l.get()).unwrap_or(1);
            return Some(MemKind::Seq(lat));
        }
    }
    None
}

pub fn memory_kinds(comp: &ir::Component) -> HashMap<ir::Id, MemKind> {
    let mut out = HashMap::new();
    for cell in comp.cells.iter() {
        let cell = cell.borrow();
        if let Some(kind) = classify_memory(&cell) {
            out.insert(cell.name(), kind);
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct MemoryAccess {
    pub memory: ir::Id,
    pub site: ir::Id,
    pub kind: AccessKind,
    pub span: u64,
    pub offset: u64,
    pub guard: ir::Guard<ir::Nothing>,
}

#[derive(Default)]
struct RoleDrive {
    driven: bool,
    guard: Option<ir::Guard<ir::Nothing>>,
    start: Option<u64>,
    span: Option<u64>,
}

#[derive(Default)]
struct PortDrive {
    addr: RoleDrive,
    write_en: RoleDrive,
    content_en: RoleDrive,
}

impl PortDrive {
    fn triggered(&self, kind: MemKind) -> bool {
        match kind {
            MemKind::Comb => self.addr.driven,
            MemKind::Seq(_) => self.content_en.driven,
        }
    }

    fn deciding(&self, kind: MemKind) -> &RoleDrive {
        if self.write_en.driven {
            &self.write_en
        } else {
            match kind {
                MemKind::Comb => &self.addr,
                MemKind::Seq(_) => &self.content_en,
            }
        }
    }
}

fn or_into(
    slot: &mut Option<ir::Guard<ir::Nothing>>,
    g: &ir::Guard<ir::Nothing>,
) {
    let combined = match slot.take() {
        None => g.clone(),
        Some(cur) if cur.is_true() => ir::Guard::True,
        Some(cur) => cur.or(g.clone()),
    };
    *slot = Some(combined);
}

fn or_default(slot: &Option<ir::Guard<ir::Nothing>>) -> ir::Guard<ir::Nothing> {
    slot.clone().unwrap_or(ir::Guard::True)
}

fn port_role(name: &str) -> Option<&'static str> {
    if name.starts_with("addr") {
        Some("addr")
    } else if name == "write_en" {
        Some("write_en")
    } else if name == "content_en" {
        Some("content_en")
    } else {
        None
    }
}

pub fn accesses_in_assignments(
    site: ir::Id,
    assigns: &[ir::Assignment<ir::Nothing>],
    mems: &HashMap<ir::Id, MemKind>,
) -> Vec<MemoryAccess> {
    let mut drives: HashMap<ir::Id, PortDrive> = HashMap::new();
    for assign in assigns {
        let dst = assign.dst.borrow();
        let cell = match &dst.parent {
            ir::PortParent::Cell(wc) => wc.upgrade(),
            _ => continue,
        };
        let mem_name = cell.borrow().name();
        if !mems.contains_key(&mem_name) {
            continue;
        }
        let role = match port_role(dst.name.id.as_str()) {
            Some(r) => r,
            None => continue,
        };
        if assign.guard.is_false() {
            continue;
        }
        let entry = drives.entry(mem_name).or_default();
        let slot = match role {
            "addr" => &mut entry.addr,
            "write_en" => &mut entry.write_en,
            "content_en" => &mut entry.content_en,
            _ => continue,
        };
        slot.driven = true;
        or_into(&mut slot.guard, &assign.guard);
    }

    let mut out = Vec::new();
    for (mem, drive) in drives {
        let kind = mems[&mem];
        if !drive.triggered(kind) {
            continue;
        }
        let ak = if drive.write_en.driven {
            AccessKind::Write
        } else {
            AccessKind::Read
        };
        let deciding = drive.deciding(kind);
        out.push(MemoryAccess {
            memory: mem,
            site,
            kind: ak,
            span: deciding.span.unwrap_or(1),
            offset: deciding.start.unwrap_or(0),
            guard: or_default(&deciding.guard),
        });
    }
    out.sort_by(|a, b| {
        (a.memory.to_string(), a.kind).cmp(&(b.memory.to_string(), b.kind))
    });
    out
}

fn strip_static_timing(
    guard: &ir::Guard<ir::StaticTiming>,
    interval: &mut Option<(u64, u64)>,
) -> ir::Guard<ir::Nothing> {
    match guard {
        ir::Guard::Info(timing) => {
            let found = timing.get_interval();
            *interval = Some(match *interval {
                Some((start, end)) => (start.min(found.0), end.max(found.1)),
                None => found,
            });
            ir::Guard::True
        }
        ir::Guard::True => ir::Guard::True,
        ir::Guard::Not(inner) => {
            ir::Guard::Not(Box::new(strip_static_timing(inner, interval)))
        }
        ir::Guard::And(left, right) => ir::Guard::And(
            Box::new(strip_static_timing(left, interval)),
            Box::new(strip_static_timing(right, interval)),
        ),
        ir::Guard::Or(left, right) => ir::Guard::Or(
            Box::new(strip_static_timing(left, interval)),
            Box::new(strip_static_timing(right, interval)),
        ),
        ir::Guard::CompOp(op, left, right) => ir::Guard::CompOp(
            op.clone(),
            ir::RRC::clone(left),
            ir::RRC::clone(right),
        ),
        ir::Guard::Port(port) => ir::Guard::Port(ir::RRC::clone(port)),
    }
}

pub fn static_accesses_in_assignments(
    site: ir::Id,
    assigns: &[ir::Assignment<ir::StaticTiming>],
    mems: &HashMap<ir::Id, MemKind>,
) -> Vec<MemoryAccess> {
    let mut drives: HashMap<ir::Id, PortDrive> = HashMap::new();
    for assign in assigns {
        if assign.guard.is_false() {
            continue;
        }
        let dst = assign.dst.borrow();
        let cell = match &dst.parent {
            ir::PortParent::Cell(wc) => wc.upgrade(),
            _ => continue,
        };
        let mem_name = cell.borrow().name();
        if !mems.contains_key(&mem_name) {
            continue;
        }
        let mut assign_interval: Option<(u64, u64)> = None;
        let guard: ir::Guard<ir::Nothing> =
            strip_static_timing(&assign.guard, &mut assign_interval);
        let entry = drives.entry(mem_name).or_default();
        let slot = match port_role(dst.name.id.as_str()) {
            Some("addr") => &mut entry.addr,
            Some("write_en") => &mut entry.write_en,
            Some("content_en") => &mut entry.content_en,
            _ => continue,
        };
        slot.driven = true;
        or_into(&mut slot.guard, &guard);
        if let Some((interval_start, interval_end)) = assign_interval {
            let width = interval_end.saturating_sub(interval_start).max(1);
            slot.start = Some(match slot.start {
                Some(current) => current.min(interval_start),
                None => interval_start,
            });
            slot.span = Some(match slot.span {
                Some(current) => current.max(width),
                None => width,
            });
        }
    }

    let mut out = Vec::new();
    for (mem, drive) in drives {
        let kind = mems[&mem];
        if !drive.triggered(kind) {
            continue;
        }
        let ak = if drive.write_en.driven {
            AccessKind::Write
        } else {
            AccessKind::Read
        };
        let deciding = drive.deciding(kind);
        out.push(MemoryAccess {
            memory: mem,
            site,
            kind: ak,
            span: deciding.span.unwrap_or(1),
            offset: deciding.start.unwrap_or(0),
            guard: or_default(&deciding.guard),
        });
    }
    out.sort_by(|a, b| {
        (a.memory.to_string(), a.kind).cmp(&(b.memory.to_string(), b.kind))
    });
    out
}
