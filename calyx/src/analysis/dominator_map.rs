use crate::ir::GetAttributes;
use crate::ir::{self};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

const NODE_ID: &str = "NODE_ID";
const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

/// Builds a Domination Map for the control program. If maps control statement
/// ids to sets of control statement ids. In the context of the domination map,
/// the id of a while loop refers to the guard condition. The begin and end id
/// of an if statement refer to the guard and "end node" of the if statement.
/// The id of invokes and enables are self explanatory. The ids of seqs and pars
/// should not be included in the map.

#[derive(Default)]
pub struct DominatorMap {
    /// Map from group names to the name of groups that dominate it
    pub map: HashMap<u64, HashSet<u64>>,
}

impl Debug for DominatorMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //must sort the hashmap and hashsets in order to get consistent ordering
        writeln!(f, "Domination Map {{")?;
        let map = self.map.clone();
        let mut vec1: Vec<(u64, HashSet<u64>)> = map.into_iter().collect();
        vec1.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        for (k, hs) in vec1.into_iter() {
            writeln!(f, "group: {:?}", k)?;
            let mut vec = hs.into_iter().collect::<Vec<_>>();
            vec.sort_unstable();
            for v in vec.iter() {
                writeln!(f, "{:?}", v)?;
            }
        }
        write!(f, "}}")
    }
}

// given a stmt, returns Some(&val) where val is the value of the s
// attribute of stmt. Returns None if no s attribute exists.
fn get_attr(stmt: &ir::Control, s: &str) -> Option<u64> {
    stmt.get_attributes().and_then(|atts| atts.get(s)).copied()
}

/// Caleb: This is a copy+ paste from the tdcc pass that I edited slightly.
/// Caleb: Should unify at some point.
/// Adds the @NODE_ID attribute to all control stmts except emtpy ones.
/// Also, for If stmts, instead of an @NODE_ID, it gets a beginning and end
/// id.
///
/// ## Example:
/// ```
/// seq { A; if cond {X} else{Y}; par { C; D; }; E }
/// ```
/// gets the labels:
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
/// and [control_exits].
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
            let mut new_state = compute_unique_ids(tbranch, cur_state);
            cur_state = new_state;
            new_state = compute_unique_ids(fbranch, cur_state);
            cur_state = new_state;
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
    pub fn new(control: &mut ir::Control) -> Self {
        compute_unique_ids(control, 0);
        let mut map = DominatorMap {
            map: HashMap::new(),
        };
        DominatorMap::build_map(control, &mut map);
        map
    }

    //given a control, gets its associated id. For if statments, gets the
    //beginning id, not the end id.
    fn get_id(c: &ir::Control) -> u64 {
        if let Some(v) = match c {
            ir::Control::If(_) => get_attr(c, BEGIN_ID),
            _ => get_attr(c, NODE_ID),
        } {
            v
        } else {
            panic!("TO DO: Empty stmt")
        }
    }

    //given a control stmt c and a key, returns true if c matches key, false
    //otherwise
    fn matches_key(c: &ir::Control, key: u64) -> bool {
        let mut ids = Vec::new();
        let first_id = DominatorMap::get_id(c);
        ids.push(first_id);
        if let Some(end) = get_attr(c, END_ID) {
            ids.push(end);
        }
        for id in ids {
            if id == key {
                return true;
            }
        }
        false
    }

    //given a control c and a pointer, finds the control statement within c, if
    // it exists. If it doesn't return None.
    pub fn get_control(id: u64, c: &ir::Control) -> Option<&ir::Control> {
        if matches!(c, ir::Control::Empty(_)) {
            return None;
        }
        if DominatorMap::matches_key(c, id) {
            return Some(c);
        }
        match c {
            ir::Control::Empty(_)
            | ir::Control::Invoke(_)
            | ir::Control::Enable(_) => None,
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    match DominatorMap::get_control(id, stmt) {
                        Some(stmt) => return Some(stmt),
                        None => (),
                    }
                }
                None
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                match DominatorMap::get_control(id, tbranch) {
                    Some(stmt) => return Some(stmt),
                    None => (),
                };
                match DominatorMap::get_control(id, fbranch) {
                    Some(stmt) => return Some(stmt),
                    None => (),
                }
                None
            }
            ir::Control::While(ir::While { body, .. }) => {
                match DominatorMap::get_control(id, body) {
                    Some(stmt) => return Some(stmt),
                    None => (),
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
                "every while/envoke/enable stmt should have a node id"
            ),
        }
    }

    //gets the "final" node in each of the controls. This is a way of getting
    //what will be the predecessor of the next node in the control sequence.
    //The one exception is if we have a while loop, there will be a predecessor
    //that occurs after the while loop. this function does not take that into account,
    //so we must deal with it later.
    fn get_final(c: &ir::Control) -> HashSet<u64> {
        let mut hs = HashSet::new();
        match c {
            ir::Control::Empty(_) => panic!("To Do: deal w/ empty controls"),
            ir::Control::Invoke(_)
            | ir::Control::Enable(_)
            | ir::Control::While(_) => {
                let id = DominatorMap::get_guaranteed_attribute(c, NODE_ID);
                hs.insert(id);
            }
            ir::Control::If(_) => {
                let id = DominatorMap::get_guaranteed_attribute(c, END_ID);
                hs.insert(id);
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                match (&stmts[..]).last() {
                    None => panic!("To Do: how to deal with empty seq?"),
                    Some(control) => return DominatorMap::get_final(control),
                }
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    let stmt_final = DominatorMap::get_final(stmt);
                    hs = hs.union(&stmt_final).copied().collect()
                }
            }
        }
        hs
    }

    //given predecessors of a node and a domination map, gets the domination
    //nodes by taking the union of all of the dominators of the predecessors.
    fn get_dominators(
        pred: &HashSet<u64>,
        d_map: &DominatorMap,
    ) -> HashSet<u64> {
        let mut pred_dominators: Vec<&HashSet<u64>> = Vec::new();
        for id in pred.iter() {
            if let Some(dominators) = d_map.map.get(id) {
                pred_dominators.push(dominators);
            }
        }
        let mut union: HashSet<u64> = HashSet::new();
        pred_dominators.iter().for_each(|set| {
            union = union.union(set).copied().collect();
        });
        union
    }

    //builds the domination map
    fn build_map(main_c: &ir::Control, d_map: &mut DominatorMap) {
        let mut og_map = d_map.map.clone();
        let empty_set: HashSet<u64> = HashSet::new();
        DominatorMap::update_map(main_c, 0, &empty_set, d_map);
        while og_map != d_map.map {
            og_map = d_map.map.clone();
            DominatorMap::update_map(main_c, 0, &empty_set, d_map);
        }
    }

    //gets the dominators of id through pred and d_map and adds
    //that value to id.
    fn update_node(pred: &HashSet<u64>, id: u64, d_map: &mut DominatorMap) {
        let mut dominators = DominatorMap::get_dominators(pred, d_map);
        dominators.insert(id);
        d_map.map.insert(id, dominators);
    }

    //takes the intersection of intersect1 and intersect2 and updates d_map to
    //store value id + (intersect1 intersect intersect2) in key id.
    fn update_intersection(
        intersect1: &HashSet<u64>,
        intersect2: &HashSet<u64>,
        id: u64,
        d_map: &mut DominatorMap,
    ) {
        let mut intersect: HashSet<u64> =
            intersect1.intersection(intersect2).copied().collect();
        intersect.insert(id);
        d_map.map.insert(id, intersect);
    }

    //Looks through each "node" in the graph and updates the dominators accordingly
    fn update_map(
        main_c: &ir::Control,
        cur_id: u64,
        pred: &HashSet<u64>,
        d_map: &mut DominatorMap,
    ) {
        let c = match DominatorMap::get_control(cur_id, main_c) {
            Some(control) => control,
            None => return (),
        };
        match c {
            ir::Control::Empty(_) => {
                unreachable!("should skip pattern matching if empty stmt")
            }
            ir::Control::Invoke(_) | ir::Control::Enable(_) => {
                DominatorMap::update_node(pred, cur_id, d_map);
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                //FIND BETTER WAY TO DO THIS. THIS IS UGLY
                let mut cur_pred = HashSet::new();
                let mut first = true;
                for stmt in stmts {
                    let id = DominatorMap::get_id(stmt);
                    if first {
                        DominatorMap::update_map(main_c, id, pred, d_map);
                        first = false;
                    } else {
                        DominatorMap::update_map(main_c, id, &cur_pred, d_map);
                    }
                    cur_pred = DominatorMap::get_final(stmt);
                }
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    let id = DominatorMap::get_id(stmt);
                    DominatorMap::update_map(main_c, id, pred, d_map);
                }
            }
            ir::Control::While(ir::While { body, .. }) => {
                //when we update the guard of the while loop, we can ignore one
                //predecessor: the last node in the body of the while loop.
                //While this is a predecessor, since the dominators of the last
                //node in the while body are either a) in the while loop, in
                //which case we know it won't be a dominator of the while guard,
                //or b) are outside the while loop, in which case we know they
                //dominate at least one of the other predecessors of the while guard,
                //since all paths into the while loop must go through the while guard
                DominatorMap::update_node(pred, cur_id, d_map);

                //updating the while body
                let body_id = DominatorMap::get_id(body);
                let mut while_guard_set = HashSet::new();
                while_guard_set.insert(cur_id);
                DominatorMap::update_map(
                    main_c,
                    body_id,
                    &while_guard_set,
                    d_map,
                );
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                DominatorMap::update_node(pred, cur_id, d_map);
                //building the predecessor set for the tbranch and fbranch
                let mut if_guard_set = HashSet::new();
                if_guard_set.insert(cur_id);

                let t_id = DominatorMap::get_id(tbranch);
                DominatorMap::update_map(main_c, t_id, &if_guard_set, d_map);
                let last_tbranch = DominatorMap::get_final(tbranch);
                let tbranch_dominators =
                    DominatorMap::get_dominators(&last_tbranch, d_map);

                let end_id = DominatorMap::get_guaranteed_attribute(c, END_ID);
                if matches!(**fbranch, ir::Control::Empty(_)) {
                    //no else stmt
                    let mut guard_dominators =
                        DominatorMap::get_dominators(pred, d_map);
                    guard_dominators.insert(cur_id);
                    DominatorMap::update_intersection(
                        &tbranch_dominators,
                        &guard_dominators,
                        end_id,
                        d_map,
                    );
                } else {
                    let f_id = DominatorMap::get_id(fbranch);
                    DominatorMap::update_map(
                        main_c,
                        f_id,
                        &if_guard_set,
                        d_map,
                    );
                    let last_fbranch = DominatorMap::get_final(fbranch);
                    let fbranch_dominators =
                        DominatorMap::get_dominators(&last_fbranch, d_map);
                    DominatorMap::update_intersection(
                        &tbranch_dominators,
                        &fbranch_dominators,
                        end_id,
                        d_map,
                    );
                }
            }
        };
    }
}
