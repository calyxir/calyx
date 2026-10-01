use crate::analysis::memory_access::{
    MemKind, MemoryAccess, accesses_in_assignments, memory_kinds,
    static_accesses_in_assignments,
};
use calyx_ir as ir;
use std::collections::{BTreeSet, HashMap};

pub struct AccessGraph {
    pub accesses: Vec<MemoryAccess>,
    pub edges: BTreeSet<(usize, usize)>,
    pub memories: Vec<ir::Id>,
}

fn windows_overlap(
    start_a: u64,
    span_a: u64,
    start_b: u64,
    span_b: u64,
) -> bool {
    let end_a = start_a + span_a.max(1);
    let end_b = start_b + span_b.max(1);
    start_a < end_b && start_b < end_a
}

impl AccessGraph {
    pub fn build(comp: &ir::Component) -> Self {
        let mems = memory_kinds(comp);
        let mut memories: Vec<ir::Id> = mems.keys().copied().collect();
        memories.sort_by_key(|memory| memory.to_string());
        let mut graph = AccessGraph {
            accesses: Vec::new(),
            edges: BTreeSet::new(),
            memories,
        };
        let control = comp.control.borrow();
        graph.visit_dynamic(&control, &mems);
        graph
    }

    fn push(&mut self, access: MemoryAccess) -> usize {
        let idx = self.accesses.len();
        self.accesses.push(access);
        idx
    }

    fn add_edge(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        if self.accesses[a].memory != self.accesses[b].memory {
            return;
        }
        let edge = if a < b { (a, b) } else { (b, a) };
        self.edges.insert(edge);
    }

    fn link_clique(&mut self, indices: &[usize]) {
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                self.add_edge(indices[i], indices[j]);
            }
        }
    }

    fn link_across(&mut self, groups: &[Vec<usize>]) {
        for i in 0..groups.len() {
            for j in (i + 1)..groups.len() {
                for &a in &groups[i] {
                    for &b in &groups[j] {
                        self.add_edge(a, b);
                    }
                }
            }
        }
    }

    fn leaf(&mut self, accesses: Vec<MemoryAccess>) -> Vec<usize> {
        let indices: Vec<usize> =
            accesses.into_iter().map(|a| self.push(a)).collect();
        self.link_clique(&indices);
        indices
    }

    fn visit_dynamic(
        &mut self,
        control: &ir::Control,
        mems: &HashMap<ir::Id, MemKind>,
    ) -> Vec<usize> {
        match control {
            ir::Control::Empty(_)
            | ir::Control::FSMEnable(_)
            | ir::Control::Invoke(_) => Vec::new(),
            ir::Control::Enable(ir::Enable { group, .. }) => {
                let group = group.borrow();
                let accesses = accesses_in_assignments(
                    group.name(),
                    &group.assignments,
                    mems,
                );
                self.leaf(accesses)
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => stmts
                .iter()
                .flat_map(|s| self.visit_dynamic(s, mems))
                .collect(),
            ir::Control::Par(ir::Par { stmts, .. }) => {
                let groups: Vec<Vec<usize>> =
                    stmts.iter().map(|s| self.visit_dynamic(s, mems)).collect();
                self.link_across(&groups);
                groups.into_iter().flatten().collect()
            }
            ir::Control::If(ir::If {
                cond,
                tbranch,
                fbranch,
                ..
            }) => {
                let mut all = Vec::new();
                if let Some(cg) = cond {
                    let cg = cg.borrow();
                    let accesses = accesses_in_assignments(
                        cg.name(),
                        &cg.assignments,
                        mems,
                    );
                    all.extend(self.leaf(accesses));
                }
                all.extend(self.visit_dynamic(tbranch, mems));
                all.extend(self.visit_dynamic(fbranch, mems));
                all
            }
            ir::Control::While(ir::While { cond, body, .. }) => {
                let mut all = Vec::new();
                if let Some(cg) = cond {
                    let cg = cg.borrow();
                    let accesses = accesses_in_assignments(
                        cg.name(),
                        &cg.assignments,
                        mems,
                    );
                    all.extend(self.leaf(accesses));
                }
                all.extend(self.visit_dynamic(body, mems));
                all
            }
            ir::Control::Repeat(ir::Repeat { body, .. }) => {
                self.visit_dynamic(body, mems)
            }
            ir::Control::Static(sc) => self
                .visit_static(sc, mems)
                .into_iter()
                .map(|(idx, _)| idx)
                .collect(),
        }
    }

    fn visit_static(
        &mut self,
        control: &ir::StaticControl,
        mems: &HashMap<ir::Id, MemKind>,
    ) -> Vec<(usize, u64)> {
        match control {
            ir::StaticControl::Empty(_) | ir::StaticControl::Invoke(_) => {
                Vec::new()
            }
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                let group = group.borrow();
                let accesses = static_accesses_in_assignments(
                    group.name(),
                    &group.assignments,
                    mems,
                );
                let indices: Vec<usize> =
                    accesses.into_iter().map(|a| self.push(a)).collect();
                self.link_clique(&indices);
                indices
                    .into_iter()
                    .map(|idx| (idx, self.accesses[idx].offset))
                    .collect()
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                let mut out = Vec::new();
                let mut offset = 0u64;
                for stmt in stmts {
                    for (idx, start) in self.visit_static(stmt, mems) {
                        out.push((idx, offset + start));
                    }
                    offset += stmt.get_latency();
                }
                out
            }
            ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                let children: Vec<Vec<(usize, u64)>> =
                    stmts.iter().map(|s| self.visit_static(s, mems)).collect();
                for i in 0..children.len() {
                    for j in (i + 1)..children.len() {
                        for &(a, start_a) in &children[i] {
                            for &(b, start_b) in &children[j] {
                                let span_a = self.accesses[a].span;
                                let span_b = self.accesses[b].span;
                                if windows_overlap(
                                    start_a, span_a, start_b, span_b,
                                ) {
                                    self.add_edge(a, b);
                                }
                            }
                        }
                    }
                }
                children.into_iter().flatten().collect()
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                let mut out = self.visit_static(tbranch, mems);
                out.extend(self.visit_static(fbranch, mems));
                out
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                self.visit_static(body, mems)
            }
        }
    }
}
