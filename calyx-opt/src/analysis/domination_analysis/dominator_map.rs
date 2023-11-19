use crate::analysis::{
    domination_analysis::{
        node_analysis::{NodeReads, NodeSearch},
        static_par_domination::StaticParDomination,
    },
    ControlId, ShareSet,
};
use calyx_ir as ir;
use ir::GenericControl;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);
const BEGIN_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::BEGIN_ID);
const END_ID: ir::Attribute = ir::Attribute::Internal(ir::InternalAttr::END_ID);

/// Builds a Domination Map for the control program. It maps nodes to sets of
/// nodes. Here is what is included as a "node" in the domination map:
/// - Invokes
/// - Enables
/// - While Guards
/// - If Guards
/// - "End" If nodes, representing the place we're at in the program after the if
/// statement has just finished. This doesn't correspond to any actual Calyx code, but is
/// just a conceptualization we use to reason about domination.
/// Note that seqs and pars will *not* be included in the domination map.
///
/// Here is the algorithm we use to build the domination map.
/// - Start with an emtpy map.
/// - Visit each node n in the control program, and set:
/// - dom(n) = {U dom(p) for each predecessor p of n} U {n}. In other words, take the
/// dominators of each predecessor of n, and union them together. Then add n to
/// this set, and set this set as the dominators of n.
/// - (Another clarification): by "predecessors" of node n we mean the set of nodes
/// that could be the most recent node executed when n begins to execute.
/// - If we visit every node of the control program and the map has not changed,
/// then we are done. If it has changed, then we visit each node again to repeat
/// the process.
///
/// The reason why we can take the union (rather than intersection) of the
/// dominators of each predecessor is because we know each predecessor of each
/// node must (rather than may) be executed.
/// There are two exceptions to this general rule, and we have special cases in
/// our algorithm to deal with them.
///
/// 1) The While Guard
/// The last node(s) in the while body are predecessor(s) of the while guard but
/// are not guaranteed to be executed. So, we can think of the while guard's
/// predecessors as being split in two groups: the "body predecessors" that are not guaranteed to
/// be executed before the while guard and the "outside predecessors" that are
/// outside the body of the while loop and are guaranteed to be executed before
/// the while loop guard.
/// Here we take:
/// dom(while guard) = U(dom(outside preds)) U {while guard}
///
/// Justification:
/// dom(while guard) is a subset of U(dom(outside preds)) U {while guard}
/// Suppose n dominates the while guard. Every path to the while guard must end in
/// 1) outside pred -> while guard OR 2) body pred -> while guard. But for choice 2)
/// we know the path was really something like outside pred -> while guard -> body
/// -> while guard... body -> while guard. Since n dominates the while guard
/// we know that it *cannot* be in the while body. Therefore, since every path to the
/// while guard is in the form outside pred -> [possibly while guard + some other
/// while body statements] -> while guard, we know that n must either dominate
/// outside pred or be the while guard itself.
///
/// dom(outside preds) U {while guard} is a subset of dom(while guard)
/// Suppose n dominates outside preds. Since we already established that every
/// path to the while guard involves going through otuside preds, we know that
/// n dominates the while guard.
///
/// 2) "End Node" of If Statements
/// In this case, *neither* of the predecessor sets (the set in the tbranch or
/// the set in the fbranch) are guaranteed to be executed.
/// Here we take:
/// dom(end node) = dom(if guard) U {end node}.
///
/// Justification:
/// dom(end node) is a subset of dom(if guard) U {end node}.
/// If n dominates the end node, then it either a) is the end node itself, or b) must
/// dominate the if guard. Justification for b)
/// Every possible path to the if guard must be followed by
/// if guard -> tbranch/fbranch -> end node. We also know that n must exist
/// outside the tbranch/fbranch (if it was inside either branch, it wouldn't
/// dominate the end node). Therefore, since we know that n must have appeared somewhere
/// before if_guard on the path to end node, we know n dominates the if guard.
///
/// dom(if guard) U {end node} is a subset of dom(end node)
/// If n dominates the if guard or is itself the end node, then it is very easy to
/// see how it will dominate the end node.
#[derive(Default)]
pub struct DominatorMap {
    /// Map from node (either invokes, enables, or if/while ports) ids to the ids of nodes that dominate it
    pub map: HashMap<u64, HashSet<u64>>,
    /// Maps ids of control stmts, to the "last" nodes in them. By "last" is meant
    /// the final node that will be executed in them. For invokes and enables, it
    /// will be themselves, for while statements it will be the while guard,
    /// and for if statements it will be the "if" nods. For pars in seqs, you
    /// have to look inside the children to see what their "last" nodes are.
    pub exits_map: HashMap<u64, HashSet<u64>>,
    /// an analysis to help domination across static pars
    /// static pars give us more precise timing guarantees and therefore allow
    /// us to more aggresively assign dominators
    pub static_par_domination: StaticParDomination,
    pub component_name: ir::Id,
}

impl Debug for DominatorMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //must sort the hashmap and hashsets in order to get consistent ordering
        writeln!(
            f,
            "The numbers in the domination map refer to the BEGIN_ID, END_ID, and NODE_ID attributes \nthat are attached to each non-empty control statement when the domination map is built. \nTo see which ID's refer to which control statement, look at the Calyx Program, which should \nbe printed along with the map when it is printed."
        )?;
        writeln!(
            f,
            "Domination Map for component \"{}\"  {{",
            self.component_name
        )?;
        let map = self.map.clone();
        let mut vec1: Vec<(u64, HashSet<u64>)> = map.into_iter().collect();
        vec1.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        for (k, hs) in vec1.into_iter() {
            write!(f, "Node: {:?} --", k)?;
            let mut vec = hs.into_iter().collect::<Vec<_>>();
            vec.sort_unstable();
            writeln!(f, " Dominators: {:?}", vec)?;
        }
        write!(f, "}}")
    }
}

#[inline]
fn get_id_static<const BEGIN: bool>(c: &ir::StaticControl) -> u64 {
    let v = match c {
        ir::StaticControl::If(_) => {
            if BEGIN {
                c.get_attribute(BEGIN_ID)
            } else {
                c.get_attribute(END_ID)
            }
        }
        _ => c.get_attribute(NODE_ID),
    };
    v.unwrap_or_else(|| unreachable!(
            "get_id() shouldn't be called on control stmts that don't have id numbering"
    ))
}

// Given a control, gets its associated id. For if statments, gets the
// beginning id if begin_id is true and end_id if begin_id is false.
// Should not be called on empty control
// statements or any other statements that don't have an id numbering.
#[inline]
fn get_id<const BEGIN: bool>(c: &ir::Control) -> u64 {
    let v = match c {
        ir::Control::If(_) | ir::Control::Static(ir::StaticControl::If(_)) => {
            if BEGIN {
                c.get_attribute(BEGIN_ID)
            } else {
                c.get_attribute(END_ID)
            }
        }
        _ => c.get_attribute(NODE_ID),
    };
    v.unwrap_or_else(|| unreachable!(
            "get_id() shouldn't be called on control stmts that don't have id numbering"
    ))
}

fn matches_key_static(sc: &ir::StaticControl, key: u64) -> bool {
    if get_id_static::<true>(sc) == key {
        return true;
    }
    //could match the end id of an if statement as well
    if let Some(end) = sc.get_attribute(END_ID) {
        key == end
    } else {
        false
    }
}

// Given a control stmt c and a key, returns true if c matches key, false
// otherwise. For if stmts return true if key matches either begin or end id.
fn matches_key(c: &ir::Control, key: u64) -> bool {
    if get_id::<true>(c) == key {
        return true;
    }
    //could match the end id of an if statement as well
    if let Some(end) = c.get_attribute(END_ID) {
        key == end
    } else {
        false
    }
}

fn get_final_static(sc: &ir::StaticControl) -> HashSet<u64> {
    let mut hs = HashSet::new();
    match sc {
        ir::StaticControl::Empty(_) => (),
        ir::StaticControl::Enable(_) | ir::StaticControl::Invoke(_) => {
            hs.insert(ControlId::get_guaranteed_attribute_static(sc, NODE_ID));
        }
        ir::StaticControl::Repeat(ir::StaticRepeat {
            body,
            num_repeats,
            ..
        }) => {
            // `Repeat 0` statements are essentially just Control::empty() stmts
            // and therefore do not have "final" nodes
            if *num_repeats != 0 {
                return get_final_static(body);
            }
        }
        ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
            return get_final_static(stmts[..].last().unwrap_or_else(|| {
                panic!(
                    "error: empty Static Seq block. TODO: Make Static Seq work on collapse-control pass."
                )
            }));
        }
        ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
            for stmt in stmts {
                let stmt_final = get_final_static(stmt);
                hs = hs.union(&stmt_final).copied().collect()
            }
        }
        ir::StaticControl::If(_) => {
            hs.insert(ControlId::get_guaranteed_attribute_static(sc, END_ID));
        }
    }
    hs
}

// Gets the "final" nodes in control c. Used to build exits_map.
fn get_final(c: &ir::Control) -> HashSet<u64> {
    let mut hs = HashSet::new();
    match c {
        ir::Control::Empty(_) => (),
        ir::Control::Invoke(_)
        | ir::Control::Enable(_)
        | ir::Control::While(_) => {
            hs.insert(ControlId::get_guaranteed_attribute(c, NODE_ID));
        }
        ir::Control::Repeat(ir::Repeat {
            body, num_repeats, ..
        }) => {
            // `Repeat 0` statements are essentially just Control::empty() stmts
            // and therefore do not have "final" nodes
            if *num_repeats != 0 {
                return get_final(body);
            }
        }
        ir::Control::If(_) => {
            hs.insert(ControlId::get_guaranteed_attribute(c, END_ID));
        }
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            return get_final(stmts[..].last().unwrap_or_else(|| {
                panic!("error: empty Seq block. Run collapse-control pass.")
            }));
        }
        ir::Control::Par(ir::Par { stmts, .. }) => {
            for stmt in stmts {
                let stmt_final = get_final(stmt);
                hs = hs.union(&stmt_final).copied().collect()
            }
        }
        ir::Control::Static(s) => return get_final_static(s),
    }
    hs
}

impl DominatorMap {
    /// Construct a domination map.
    pub fn new(control: &mut ir::Control, component_name: ir::Id) -> Self {
        ControlId::compute_unique_ids(control, 0, true);
        let mut map = DominatorMap {
            map: HashMap::new(),
            exits_map: HashMap::new(),
            static_par_domination: StaticParDomination::new(
                control,
                component_name,
            ),
            component_name,
        };
        map.build_exit_map(control);
        map.build_map(control);
        map
    }

    fn build_exit_map_static(&mut self, sc: &ir::StaticControl) {
        match sc {
            ir::StaticControl::Enable(_) | ir::StaticControl::Invoke(_) => {
                let id =
                    ControlId::get_guaranteed_attribute_static(sc, NODE_ID);
                self.exits_map.insert(id, HashSet::from([id]));
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                let id =
                    ControlId::get_guaranteed_attribute_static(sc, NODE_ID);
                self.exits_map.insert(id, get_final_static(sc));
                self.build_exit_map_static(body);
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
            | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                for stmt in stmts {
                    self.build_exit_map_static(stmt);
                }
                let id =
                    ControlId::get_guaranteed_attribute_static(sc, NODE_ID);
                self.exits_map.insert(id, get_final_static(sc));
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                let begin_id =
                    ControlId::get_guaranteed_attribute_static(sc, BEGIN_ID);
                let end_id =
                    ControlId::get_guaranteed_attribute_static(sc, END_ID);
                self.exits_map.insert(begin_id, HashSet::from([end_id]));
                self.exits_map.insert(end_id, HashSet::from([end_id]));
                self.build_exit_map_static(tbranch);
                self.build_exit_map_static(fbranch);
            }
            ir::StaticControl::Empty(_) => (),
        }
    }

    // Builds the "exit map" of c. This is getting what will be the final "node"
    // executed in c.
    fn build_exit_map(&mut self, c: &ir::Control) {
        match c {
            ir::Control::Empty(_) => (),
            ir::Control::Invoke(_) | ir::Control::Enable(_) => {
                let id = ControlId::get_guaranteed_attribute(c, NODE_ID);
                self.exits_map.insert(id, HashSet::from([id]));
            }
            ir::Control::While(ir::While { body, .. }) => {
                let id = ControlId::get_guaranteed_attribute(c, NODE_ID);
                self.exits_map.insert(id, HashSet::from([id]));
                self.build_exit_map(body);
            }
            ir::Control::Repeat(ir::Repeat { body, .. }) => {
                let id = ControlId::get_guaranteed_attribute(c, NODE_ID);
                self.exits_map.insert(id, get_final(body));
                self.build_exit_map(body);
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                let begin_id = ControlId::get_guaranteed_attribute(c, BEGIN_ID);
                let end_id = ControlId::get_guaranteed_attribute(c, END_ID);
                self.exits_map.insert(begin_id, HashSet::from([end_id]));
                self.exits_map.insert(end_id, HashSet::from([end_id]));
                self.build_exit_map(tbranch);
                self.build_exit_map(fbranch);
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    self.build_exit_map(stmt);
                }
                let id = ControlId::get_guaranteed_attribute(c, NODE_ID);
                self.exits_map.insert(id, get_final(c));
            }
            ir::Control::Static(sc) => self.build_exit_map_static(sc),
        }
    }

    // Builds the domination map by running update_map() until the map
    // stops changing.
    fn build_map(&mut self, main_c: &mut ir::Control) {
        let mut og_map = self.map.clone();
        self.update_map(main_c, 0, &HashSet::new());
        while og_map != self.map {
            og_map = self.map.clone();
            self.update_map(main_c, 0, &HashSet::new());
        }
        self.update_static_dominators();
    }

    // updates static dominators based on self.static_par_domination
    // this can more aggresively add dominators to the map by
    // using the timing guarantees of static par
    fn update_static_dominators(&mut self) {
        let new_static_domminators =
            self.static_par_domination.get_static_dominators();
        for (node_id, node_dominators) in new_static_domminators {
            let cur_dominators = self.map.entry(node_id).or_default();
            cur_dominators.extend(node_dominators);
        }
    }

    // Given an id and its predecessors pred, and a domination map d_map, updates
    // d_map accordingly (i.e. the union of all dominators of the predecessors
    // plus itself).
    fn update_node(&mut self, pred: &HashSet<u64>, id: u64) {
        let mut union: HashSet<u64> = HashSet::new();
        for id in pred.iter() {
            if let Some(dominators) = self.map.get(id) {
                union = union.union(dominators).copied().collect();
            }
        }
        union.insert(id);
        self.map.insert(id, union);
    }

    fn update_map_static(
        &mut self,
        main_sc: &ir::StaticControl,
        cur_id: u64,
        pred: &HashSet<u64>,
    ) {
        match Self::get_static_control(cur_id, main_sc) {
            Some(GenericControl::Dynamic(_)) => {
                unreachable!("should never get dynamic from get_static_control")
            }
            None => (),
            Some(GenericControl::Static(sc)) => match sc {
                ir::StaticControl::Empty(_) => (),
                ir::StaticControl::Enable(_) | ir::StaticControl::Invoke(_) => {
                    self.update_node(pred, cur_id);
                }
                ir::StaticControl::Repeat(ir::StaticRepeat {
                    body,
                    num_repeats,
                    ..
                }) => {
                    if *num_repeats != 0 {
                        let body_id = get_id_static::<true>(body);
                        self.update_map_static(main_sc, body_id, pred);
                    }
                }
                ir::StaticControl::Seq(ir::StaticSeq { stmts, .. }) => {
                    let mut p = pred;
                    let mut nxt: HashSet<u64>;
                    for stmt in stmts {
                        let id = get_id_static::<true>(stmt);
                        self.update_map_static(main_sc, id, p);
                        // updating the predecessors for the next stmt we iterate
                        nxt = self
                            .exits_map
                            .get(&id)
                            .unwrap_or(
                                // If the exits map is empty, then it means the
                                // current stmt is `Repeat 0`/Empty.
                                // So the predecessors for the nxt stmt are the
                                // same as the predecessors for the current stmt.
                                pred,
                            )
                            .clone();
                        p = &nxt;
                    }
                }
                ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                    for stmt in stmts {
                        let id = get_id_static::<true>(stmt);
                        self.update_map_static(main_sc, id, pred);
                    }
                }
                ir::StaticControl::If(ir::StaticIf {
                    tbranch,
                    fbranch,
                    ..
                }) => {
                    //updating the if guard
                    self.update_node(pred, cur_id);

                    //building a set w/ just the if_guard id in it
                    let if_guard_set = HashSet::from([cur_id]);

                    //updating the tbranch
                    let t_id = get_id_static::<true>(tbranch);
                    self.update_map_static(main_sc, t_id, &if_guard_set);

                    // If the false branch is present, update the map
                    if !matches!(**fbranch, ir::StaticControl::Empty(_)) {
                        let f_id = get_id_static::<true>(fbranch);
                        self.update_map_static(main_sc, f_id, &if_guard_set);
                    }

                    let end_id =
                        ControlId::get_guaranteed_attribute_static(sc, END_ID);
                    self.update_node(&if_guard_set, end_id)
                }
            },
        }
    }

    // Looks through each "node" in the "graph" and updates the dominators accordingly
    fn update_map(
        &mut self,
        main_c: &ir::Control,
        cur_id: u64,
        pred: &HashSet<u64>,
    ) {
        match Self::get_control(cur_id, main_c) {
            None => (),
            Some(GenericControl::Dynamic(c)) => {
                match c {
                    ir::Control::Empty(_) => {
                        unreachable!(
                            "should not pattern match agaisnt empty in update_map()"
                        )
                    }
                    ir::Control::Invoke(_)
                    | ir::Control::Enable(_) => {
                        self.update_node(pred, cur_id);
                    }
                    ir::Control::Seq(ir::Seq { stmts, .. }) => {
                        let mut p = pred;
                        let mut nxt: HashSet<u64>;
                        for stmt in stmts {
                            let id = get_id::<true>(stmt);
                            self.update_map(main_c, id, p);
                            nxt = self
                                .exits_map
                                .get(&id)
                                .unwrap_or(pred
                                    // If the exits map is empty, then it means the
                                    // current stmt is `Repeat 0`/Empty.
                                    // So the predecessors for the nxt stmt are the
                                    // same as the predecessors for the current stmt
                                ).clone();
                            p = &nxt;
                        }
                    }
                    ir::Control::Par(ir::Par { stmts, .. }) => {
                        for stmt in stmts {
                            let id = get_id::<true>(stmt);
                            self.update_map(main_c, id, pred);
                        }
                    }
                    ir::Control::Repeat(ir::Repeat { body, num_repeats, .. }) => {
                        if *num_repeats != 0 {
                            let body_id = get_id::<true>(body);
                            self.update_map(main_c, body_id, pred);
                        }
                    }
                    // Keep in mind that NODE_IDs attached to while loops/if statements
                    // refer to the while/if guard, and as we pattern match against a while
                    // or if statement, the control statement refers to the "guard",
                    // which includes their combinational group and the conditional port
                    // So (for example) if a while loop has NODE_ID = 10, then "node 10"
                    // refers to the while guard-- comb group and conditional port-- but not the body.
                    ir::Control::While(ir::While { body, .. }) => {
                        self.update_node(pred, cur_id);
                        // updating the while body
                        let body_id = get_id::<true>(body);
                        self.update_map(
                            main_c,
                            body_id,
                            &HashSet::from([cur_id]),
                        );
                    }
                    ir::Control::If(ir::If {
                        tbranch, fbranch, ..
                    }) => {
                        //updating the if guard
                        self.update_node(pred, cur_id);

                        //building a set w/ just the if_guard id in it
                        let if_guard_set = HashSet::from([cur_id]);

                        //updating the tbranch
                        let t_id = get_id::<true>(tbranch);
                        self.update_map(main_c, t_id, &if_guard_set);

                        // If the false branch is present, update the map
                        if !matches!(**fbranch, ir::Control::Empty(_)) {
                            let f_id = get_id::<true>(fbranch);
                            self.update_map(main_c, f_id, &if_guard_set);
                        }

                        let end_id =
                            ControlId::get_guaranteed_attribute(c, END_ID);
                        self.update_node(&if_guard_set, end_id)
                    }
                    ir::Control::Static(_) => panic!("when matching c in GenericControl::Dynamic(c), c shouldn't be Static Control")
                };
            }
            Some(GenericControl::Static(sc)) => {
                let static_id = get_id_static::<true>(sc);
                self.update_map_static(sc, static_id, pred);
            }
        }
    }

    pub fn get_static_control(
        id: u64,
        sc: &ir::StaticControl,
    ) -> Option<GenericControl> {
        if matches!(sc, ir::StaticControl::Empty(_)) {
            return None;
        }
        if matches_key_static(sc, id) {
            return Some(GenericControl::from(sc));
        };
        match sc {
            ir::StaticControl::Empty(_)
            | ir::StaticControl::Enable(_)
            | ir::StaticControl::Invoke(_) => None,
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                Self::get_static_control(id, body)
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
            | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                for stmt in stmts {
                    match Self::get_static_control(id, stmt) {
                        None => (),
                        Some(GenericControl::Dynamic(_)) => {
                            unreachable!("Got a GenericControl::Dynamic when we called get_static_control")
                        }
                        Some(GenericControl::Static(sc)) => {
                            return Some(GenericControl::from(sc))
                        }
                    }
                }
                None
            }
            ir::StaticControl::If(ir::StaticIf {
                tbranch, fbranch, ..
            }) => {
                match Self::get_static_control(id, tbranch) {
                    Some(GenericControl::Dynamic(_)) => {
                        unreachable!("Got a GenericControl::Dynamic when we called get_static_control")
                    }
                    Some(GenericControl::Static(sc)) => {
                        return Some(GenericControl::from(sc))
                    }
                    None => (),
                }
                match Self::get_static_control(id, fbranch) {
                    Some(GenericControl::Dynamic(_)) => {
                        unreachable!("Got a GenericControl::Dynamic when we called get_static_control")
                    }
                    Some(GenericControl::Static(sc)) => {
                        return Some(GenericControl::from(sc))
                    }
                    None => (),
                };
                None
            }
        }
    }

    /// Given a control c and an id, finds the control statement within c that
    /// has id, if it exists. If it doesn't, return None.
    pub fn get_control(id: u64, c: &ir::Control) -> Option<GenericControl> {
        if matches!(c, ir::Control::Empty(_)) {
            return None;
        }
        if matches_key(c, id) {
            return Some(GenericControl::from(c));
        }
        match c {
            ir::Control::Empty(_)
            | ir::Control::Invoke(_)
            | ir::Control::Enable(_) => None,
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    match Self::get_control(id, stmt) {
                        None => (),
                        Some(GenericControl::Dynamic(c)) => {
                            return Some(GenericControl::from(c))
                        }
                        Some(GenericControl::Static(sc)) => {
                            return Some(GenericControl::from(sc))
                        }
                    }
                }
                None
            }
            ir::Control::Repeat(ir::Repeat { body, .. }) => {
                Self::get_control(id, body)
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                match Self::get_control(id, tbranch) {
                    Some(GenericControl::Dynamic(c)) => {
                        return Some(GenericControl::from(c))
                    }
                    Some(GenericControl::Static(sc)) => {
                        return Some(GenericControl::from(sc))
                    }
                    None => (),
                }
                match Self::get_control(id, fbranch) {
                    Some(GenericControl::Dynamic(c)) => {
                        return Some(GenericControl::from(c))
                    }
                    Some(GenericControl::Static(sc)) => {
                        return Some(GenericControl::from(sc))
                    }
                    None => (),
                };
                None
            }
            ir::Control::While(ir::While { body, .. }) => {
                Self::get_control(id, body)
            }
            ir::Control::Static(sc) => Self::get_static_control(id, sc),
        }
    }

    // Given a set of nodes, gets the control in main_control that corresponds
    // to the node. If there is a node in the set not corresponding to a control
    // statement in main_control, then it gives an unreachable! error.
    // Returns two vectors: controls, static_controls
    // (the dynamic and static nodes)
    pub fn get_control_nodes<'a>(
        nodes: &HashSet<u64>,
        main_control: &'a ir::Control,
    ) -> (Vec<&'a ir::Control>, Vec<&'a ir::StaticControl>) {
        let mut controls: Vec<&ir::Control> = Vec::new();
        let mut static_controls: Vec<&ir::StaticControl> = Vec::new();
        for node in nodes {
            match Self::get_control(*node, main_control) {
                Some(GenericControl::Static(sc)) => static_controls.push(sc),
                Some(GenericControl::Dynamic(c)) => controls.push(c),
                None => {
                    unreachable!("No control statement for ID {}", node)
                }
            }
        }
        (controls, static_controls)
    }

    // Gets the reads of shareable cells in node
    // Assumes the control statements in comp have been given NODE_IDs in the same
    // style of the domination map NODE_ID stuff.
    pub fn get_node_reads(
        node: &u64,
        comp: &mut ir::Component,
        shareset: &ShareSet,
    ) -> HashSet<ir::Id> {
        NodeReads::get_reads_of_node(node, comp, shareset)
    }

    // Returns whether key is guaranteed to be written in at least one of nodes
    // Assumes the control statements in comp have been given NODE_IDs in the same
    // style of the domination map NODE_ID stuff.
    pub fn key_written_guaranteed(
        key: ir::Id,
        nodes: &HashSet<u64>,
        comp: &mut ir::Component,
    ) -> bool {
        let search_struct = NodeSearch::new(key);
        search_struct.is_written_guaranteed(nodes, comp)
    }
}
