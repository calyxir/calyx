use crate::ir::GetAttributes;
use crate::ir::{self};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

const NODE_ID: &str = "NODE_ID";
const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

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
    /// Map from group names to the name of groups that dominate it
    pub map: HashMap<u64, HashSet<u64>>,
    /// Maps ids of control stmts, to the "last" nodes in them. By "last" is meant
    /// the final node that will be executed in them. For invokes and enables, it
    /// will be themselves, for while statements it will be the while guard,
    /// and for if statements it will be the "if" nods. For pars in seqs, you
    /// have to look inside the children to see what their "last" nodes are.  
    pub exits_map: HashMap<u64, HashSet<u64>>,
    pub component_name: String,
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

// Given a control stmt, returns Some(val) where val is the value of attribute s
// of stmt. Returns None if no s attribute exists.
fn get_attr(stmt: &ir::Control, s: &str) -> Option<u64> {
    stmt.get_attributes().and_then(|atts| atts.get(s)).copied()
}

/// Caleb Note: This is a copy+ paste from the tdcc pass that I edited slightly.
/// We should unify at some point.
/// Adds the @NODE_ID attribute to all control stmts except emtpy ones.
/// Also, for If stmts, instead of an @NODE_ID, it gets a beginning and end
/// id.
///
/// ## Example:
/// ```
/// seq { A; if cond {X} else{Y}; par { C; D; }; E }
/// ```
///
/// gets the labels:
///
/// ```
/// @NODE_ID(0)seq {
///   @NODE_ID(1) A;
///   @BEGIN_ID(2) @END_ID(5) if cond {
///     @NODE_ID(3) X
///   }
///   else{
///     @NODE_ID(4) Y
///   }
///   @NODE_ID(6) par {
///     @NODE_ID(7) C;
///     @NODE_ID(8) D;
///   }
///   @NODE_ID(9) E;
/// }
///
/// These identifiers are used by the compilation methods [calculate_states_recur]
/// and [control_exits]
fn compute_unique_ids(con: &mut ir::Control, mut cur_state: u64) -> u64 {
    match con {
        ir::Control::Enable(ir::Enable { attributes, .. })
        | ir::Control::Invoke(ir::Invoke { attributes, .. }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state + 1
        }
        ir::Control::Par(ir::Par {
            stmts, attributes, ..
        })
        | ir::Control::Seq(ir::Seq {
            stmts, attributes, ..
        }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state += 1;
            stmts.iter_mut().for_each(|stmt| {
                let new_state = compute_unique_ids(stmt, cur_state);
                cur_state = new_state;
            });
            cur_state
        }
        ir::Control::If(ir::If {
            tbranch,
            fbranch,
            attributes,
            ..
        }) => {
            attributes.insert(BEGIN_ID, cur_state);
            cur_state += 1;
            cur_state = compute_unique_ids(tbranch, cur_state);
            cur_state = compute_unique_ids(fbranch, cur_state);
            attributes.insert(END_ID, cur_state);
            cur_state + 1
        }
        ir::Control::While(ir::While {
            body, attributes, ..
        }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state += 1;
            cur_state = compute_unique_ids(body, cur_state);
            cur_state
        }
        ir::Control::Empty(_) => cur_state,
    }
}

impl DominatorMap {
    /// Construct a domination map.
    pub fn new(control: &mut ir::Control, component_name: String) -> Self {
        compute_unique_ids(control, 0);
        let mut exits_map = HashMap::new();
        Self::build_exit_map(control, &mut exits_map);
        let mut map = DominatorMap {
            map: HashMap::new(),
            exits_map,
            component_name,
        };
        Self::build_map(control, &mut map);
        map
    }

    //Builds the domination map by running update_map() until the map
    //stops changing.
    fn build_map(main_c: &ir::Control, d_map: &mut DominatorMap) {
        let mut og_map = d_map.map.clone();
        let empty_set: HashSet<u64> = HashSet::new();
        Self::update_map(main_c, 0, &empty_set, d_map);
        while og_map != d_map.map {
            og_map = d_map.map.clone();
            Self::update_map(main_c, 0, &empty_set, d_map);
        }
    }

    //Given a control, gets its associated id. For if statments, gets the
    //beginning id if begin_id is true and end_id if begin_id is false.
    //Should not be called on empty control
    //statements or any other statements that don't have an id numbering.
    fn get_id(c: &ir::Control, begin_id: bool) -> u64 {
        if let Some(v) = match c {
            ir::Control::If(_) => {
                if begin_id {
                    get_attr(c, BEGIN_ID)
                } else {
                    get_attr(c, END_ID)
                }
            }
            _ => get_attr(c, NODE_ID),
        } {
            v
        } else {
            unreachable!(
                "get_id() shouldn't be called on control stmts that don't have id numbering"
            )
        }
    }

    //given a control stmt c and a key, returns true if c matches key, false
    //otherwise. For if stmts return true if key matches either begin or end id.
    fn matches_key(c: &ir::Control, key: u64) -> bool {
        let mut ids = vec![Self::get_id(c, true)];
        if let Some(end) = get_attr(c, END_ID) {
            ids.push(end);
        }
        ids.contains(&key)
    }

    /// Given a control c and an id, finds the control statement within c that
    /// has id, if it exists. If it doesn't, return None.
    pub fn get_control(id: u64, c: &ir::Control) -> Option<&ir::Control> {
        if matches!(c, ir::Control::Empty(_)) {
            return None;
        }
        if Self::matches_key(c, id) {
            return Some(c);
        }
        match c {
            ir::Control::Empty(_)
            | ir::Control::Invoke(_)
            | ir::Control::Enable(_) => None,
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    if let Some(stmt) = Self::get_control(id, stmt) {
                        return Some(stmt);
                    }
                }
                None
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                //probably a better way to do this...
                if let Some(stmt) = Self::get_control(id, tbranch) {
                    Some(stmt)
                } else if let Some(stmt) = Self::get_control(id, fbranch) {
                    Some(stmt)
                } else {
                    None
                }
            }
            ir::Control::While(ir::While { body, .. }) => {
                if let Some(stmt) = Self::get_control(id, body) {
                    return Some(stmt);
                }
                None
            }
        }
    }

    //gets attribute s from c, panics otherwise. Should be used when you know
    //that c has attribute s.
    fn get_guaranteed_attribute(c: &ir::Control, s: &str) -> u64 {
        match get_attr(c, s) {
            Some(v) => v,
            None => unreachable!(
                "called get_guaranteed_attribute, meaning we had to be sure it had the id"
            ),
        }
    }

    //Builds the "exit map" of c. This is getting what will be the final "node"
    //executed in c.
    fn build_exit_map(
        c: &ir::Control,
        final_map: &mut HashMap<u64, HashSet<u64>>,
    ) {
        match c {
            ir::Control::Empty(_) => (),
            ir::Control::Invoke(_) | ir::Control::Enable(_) => {
                let id = Self::get_guaranteed_attribute(c, NODE_ID);
                final_map.insert(id, HashSet::from([id]));
            }
            ir::Control::While(ir::While { body, .. }) => {
                let id = Self::get_guaranteed_attribute(c, NODE_ID);
                final_map.insert(id, HashSet::from([id]));
                Self::build_exit_map(body, final_map);
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                let begin_id = Self::get_guaranteed_attribute(c, BEGIN_ID);
                let end_id = Self::get_guaranteed_attribute(c, END_ID);
                final_map.insert(begin_id, HashSet::from([end_id]));
                final_map.insert(end_id, HashSet::from([end_id]));
                Self::build_exit_map(tbranch, final_map);
                Self::build_exit_map(fbranch, final_map);
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    Self::build_exit_map(stmt, final_map);
                }
                let id = Self::get_guaranteed_attribute(c, NODE_ID);
                final_map.insert(id, Self::get_final(c));
            }
        }
    }

    //Gets the "final" nodes in control c. This useful for getting
    //what will be the predecessors of the next node in the control sequence.
    fn get_final(c: &ir::Control) -> HashSet<u64> {
        let mut hs = HashSet::new();
        match c {
            ir::Control::Empty(_) => panic!("To Do: deal w/ empty controls"),
            ir::Control::Invoke(_)
            | ir::Control::Enable(_)
            | ir::Control::While(_) => {
                hs.insert(Self::get_guaranteed_attribute(c, NODE_ID));
            }
            ir::Control::If(_) => {
                hs.insert(Self::get_guaranteed_attribute(c, END_ID));
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                match (&stmts[..]).last() {
                    None => panic!("error: empty Seq block. Run ___ "),
                    Some(control) => return Self::get_final(control),
                }
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    let stmt_final = Self::get_final(stmt);
                    hs = hs.union(&stmt_final).copied().collect()
                }
            }
        }
        hs
    }

    //Given an id and its predecessors pred, and a domination map d_map, updates
    //d_map accordingly (i.e. the union of all dominators of the predecessors
    //plus itself).
    fn update_node(pred: &HashSet<u64>, id: u64, d_map: &mut DominatorMap) {
        let mut union: HashSet<u64> = HashSet::new();
        for id in pred.iter() {
            if let Some(dominators) = d_map.map.get(id) {
                union = union.union(dominators).copied().collect();
            }
        }
        union.insert(id);
        d_map.map.insert(id, union);
    }

    //Looks through each "node" in the "graph" and updates the dominators accordingly
    fn update_map(
        main_c: &ir::Control,
        cur_id: u64,
        pred: &HashSet<u64>,
        d_map: &mut DominatorMap,
    ) {
        let c = match Self::get_control(cur_id, main_c) {
            Some(control) => control,
            None => return,
        };
        match c {
            ir::Control::Empty(_) => {
                unreachable!(
                    "should not pattern match agaisnt empty in update_map()"
                )
            }
            ir::Control::Invoke(_) | ir::Control::Enable(_) => {
                Self::update_node(pred, cur_id, d_map);
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                //Could try to think a way of doing it w/o this first stuff
                let mut first = true;
                let mut prev_id = cur_id;
                for stmt in stmts {
                    let id = Self::get_id(stmt, true);
                    if first {
                        Self::update_map(main_c, id, pred, d_map);
                        first = false;
                    } else {
                        Self::update_map(
                            main_c,
                            id,
                            &d_map
                                .exits_map
                                .get(&prev_id)
                                .unwrap_or_else(|| {
                                    unreachable!(
                                        "exit node map does not have value for {}",
                                        prev_id
                                    )
                                })
                                .clone(),
                            d_map,
                        );
                    }
                    prev_id = Self::get_id(stmt, false);
                }
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    let id = Self::get_id(stmt, true);
                    Self::update_map(main_c, id, pred, d_map);
                }
            }
            ir::Control::While(ir::While { body, .. }) => {
                Self::update_node(pred, cur_id, d_map);

                //updating the while body
                let body_id = Self::get_id(body, true);
                let mut while_guard_set = HashSet::new();
                while_guard_set.insert(cur_id);
                Self::update_map(main_c, body_id, &while_guard_set, d_map);
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                //updating the if guard
                Self::update_node(pred, cur_id, d_map);

                //building a set w/ just the if_guard id in it
                let mut if_guard_set = HashSet::new();
                if_guard_set.insert(cur_id);

                //updating the tbranch
                let t_id = Self::get_id(tbranch, true);
                Self::update_map(main_c, t_id, &if_guard_set, d_map);

                if !matches!(**fbranch, ir::Control::Empty(_)) {
                    let f_id = Self::get_id(fbranch, true);
                    Self::update_map(main_c, f_id, &if_guard_set, d_map);
                }

                let end_id = Self::get_guaranteed_attribute(c, END_ID);
                Self::update_node(&if_guard_set, end_id, d_map)
            }
        };
    }
}
