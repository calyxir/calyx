use crate::analysis::access_simultaneity::AccessGraph;
use crate::analysis::bank_report::{
    AccessSite, BankAssignment, BankReport, MemoryBanks, MemoryConflict,
};
use calyx_ir as ir;
use std::collections::{BTreeMap, BTreeSet};

/// Finds memory accesses that can happen in the same cycle.
///
/// A memory is a cell of a `comb_mem_d*` or `seq_mem_d*` primitive. The
/// analysis walks the control program of a component, works out which
/// accesses to each memory are simultaneous, and reports the resulting
/// conflicts, the minimum number of banks that removes them, a greedy bank
/// assignment, and the dependences carried across loop iterations.
///
/// Accesses in different threads of a `par` are simultaneous, while `seq`
/// children, successive loop iterations and the arms of an `if` are not.
/// Inside a static `par`, two accesses are simultaneous only if their cycles
/// overlap. Accesses under complementary guards never conflict.
pub struct BankConflictAnalysis;

impl BankConflictAnalysis {
    /// Analyzes the memory accesses of `comp`.
    pub fn analyze(comp: &ir::Component) -> BankReport {
        let graph = AccessGraph::build(comp);
        let dependences =
            crate::analysis::loop_dependence::loop_dependences(comp);
        Self::from_graph(&graph, dependences)
    }

    fn from_graph(
        graph: &AccessGraph,
        dependences: Vec<crate::analysis::bank_report::MemoryDependence>,
    ) -> BankReport {
        let mut conflict_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
        for &(a, b) in &graph.edges {
            let ga = &graph.accesses[a].guard;
            let gb = &graph.accesses[b].guard;
            if !guards_disjoint(ga, gb) {
                conflict_edges.insert((a, b));
            }
        }

        let mut conflicts = Vec::new();
        for &(a, b) in &conflict_edges {
            let left = &graph.accesses[a];
            let right = &graph.accesses[b];
            conflicts.push(MemoryConflict::new(
                left.memory,
                AccessSite::new(left.site, left.kind),
                AccessSite::new(right.site, right.kind),
            ));
        }

        let mut by_mem: BTreeMap<String, (ir::Id, Vec<usize>)> =
            BTreeMap::new();
        for &memory in &graph.memories {
            by_mem.insert(memory.to_string(), (memory, Vec::new()));
        }
        for (idx, acc) in graph.accesses.iter().enumerate() {
            by_mem
                .entry(acc.memory.to_string())
                .or_insert((acc.memory, Vec::new()))
                .1
                .push(idx);
        }

        let mut banks = Vec::new();
        let mut assignments = Vec::new();
        for (_, (mem, mut nodes)) in by_mem {
            let clique = max_clique(&nodes, &conflict_edges);
            banks.push(MemoryBanks {
                memory: mem,
                min_banks: (clique.max(1)) as u64,
            });
            nodes.sort_by(|&a, &b| {
                let ka = (
                    graph.accesses[a].site.to_string(),
                    graph.accesses[a].kind,
                );
                let kb = (
                    graph.accesses[b].site.to_string(),
                    graph.accesses[b].kind,
                );
                ka.cmp(&kb)
            });
            let coloring = greedy_color(&nodes, &conflict_edges);
            for (pos, &node) in nodes.iter().enumerate() {
                let acc = &graph.accesses[node];
                assignments.push(BankAssignment {
                    memory: mem,
                    site: AccessSite::new(acc.site, acc.kind),
                    bank: coloring[pos],
                });
            }
        }

        BankReport::from_parts(conflicts, banks, assignments, dependences)
    }
}

fn greedy_color(nodes: &[usize], edges: &BTreeSet<(usize, usize)>) -> Vec<u64> {
    let mut color = vec![0u64; nodes.len()];
    for i in 0..nodes.len() {
        let mut used = std::collections::BTreeSet::new();
        for j in 0..i {
            let (a, b) = (nodes[i], nodes[j]);
            let edge = if a < b { (a, b) } else { (b, a) };
            if edges.contains(&edge) {
                used.insert(color[j]);
            }
        }
        let mut c = 0u64;
        while used.contains(&c) {
            c += 1;
        }
        color[i] = c;
    }
    color
}

fn port_eq(a: &ir::RRC<ir::Port>, b: &ir::RRC<ir::Port>) -> bool {
    let a = a.borrow();
    let b = b.borrow();
    a.name == b.name && a.get_parent_name() == b.get_parent_name()
}

fn guard_equiv(a: &ir::Guard<ir::Nothing>, b: &ir::Guard<ir::Nothing>) -> bool {
    match (a, b) {
        (ir::Guard::True, ir::Guard::True) => true,
        (ir::Guard::Not(x), ir::Guard::Not(y)) => guard_equiv(x, y),
        (ir::Guard::And(x1, x2), ir::Guard::And(y1, y2)) => {
            guard_equiv(x1, y1) && guard_equiv(x2, y2)
        }
        (ir::Guard::Or(x1, x2), ir::Guard::Or(y1, y2)) => {
            guard_equiv(x1, y1) && guard_equiv(x2, y2)
        }
        (ir::Guard::Port(p), ir::Guard::Port(q)) => port_eq(p, q),
        (ir::Guard::CompOp(o1, x1, y1), ir::Guard::CompOp(o2, x2, y2)) => {
            o1 == o2 && port_eq(x1, x2) && port_eq(y1, y2)
        }
        _ => false,
    }
}

fn guards_disjoint(
    a: &ir::Guard<ir::Nothing>,
    b: &ir::Guard<ir::Nothing>,
) -> bool {
    if let ir::Guard::Not(inner) = a
        && guard_equiv(inner, b)
    {
        return true;
    }
    if let ir::Guard::Not(inner) = b
        && guard_equiv(inner, a)
    {
        return true;
    }
    false
}

fn max_clique(nodes: &[usize], edges: &BTreeSet<(usize, usize)>) -> usize {
    let n = nodes.len();
    if n == 0 {
        return 0;
    }
    let mut adj = vec![vec![false; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (nodes[i], nodes[j]);
            let edge = if a < b { (a, b) } else { (b, a) };
            if edges.contains(&edge) {
                adj[i][j] = true;
                adj[j][i] = true;
            }
        }
    }

    let all: Vec<usize> = (0..n).collect();
    let mut best = 1usize;
    expand(&adj, &mut Vec::new(), all, &mut best);
    best
}

fn expand(
    adj: &[Vec<bool>],
    current: &mut Vec<usize>,
    candidates: Vec<usize>,
    best: &mut usize,
) {
    if candidates.is_empty() {
        let reached = current.len();
        if reached > *best {
            *best = reached;
        }
        return;
    }
    if current.len() + candidates.len() <= *best {
        return;
    }
    for (i, &v) in candidates.iter().enumerate() {
        let next: Vec<usize> = candidates[(i + 1)..]
            .iter()
            .copied()
            .filter(|&u| adj[v][u])
            .collect();
        current.push(v);
        expand(adj, current, next, best);
        current.pop();
    }
}
