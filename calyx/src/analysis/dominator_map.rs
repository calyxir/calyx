use crate::ir::GetAttributes;
use crate::ir::{self};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

const NODE_ID: &str = "NODE_ID";
const BEGIN_ID: &str = "BEGIN_ID";
const END_ID: &str = "END_ID";

/// Builds a Domination Map for the control program.
///
/// Note about NODE_ID, BEGIN_ID, and END_ID
/// These are attributes that are attached to control statements, so that we can
/// easily identify them. In a given Calyx component, we attach
/// a unique NODE_ID to each Seq, Par, While, Enable, and Invoke,
/// and we attach a BEGIN_ID and END_ID to each if control statement. There
/// should be no repeats among ids, even between BEGIN_ID, END_ID, and NODE_ID.
/// In other words, if some control stmt in a component gets NODE_ID = 10, then
/// no other control statement in the component can have NODE_ID, BEGIN_ID, or
/// END_ID equal to 10. We assign IDs by recursively going through the control of a
/// program, making sure to attach a unique id to each (non-empty) control
/// statement in the program.
///
/// In the context of the domination_map, which maps u64s (ID of a control stmt)
/// to sets of u64s (IDs of the control stmts that dominate it), the ids represent
/// what would be the nodes of the CFG. For this reason, the ids of seqs and pars
/// should never make it into the domination map (they are used, however, to help
/// build the domination map). Furthermore, the NODE_ID of a while statement
/// refers to the while guard, not the body. So if the NODE_ID corresponding to
/// some while statement dominates a node, we are only saying that the guard
/// dominates it, not the entire body of the while. Similarly, the BEGIN_ID
/// of an if stmt, refers to the if guard, not either of its branches.  
/// The END_ID of an if statement refers to a node that occurs after
/// the if stmt finishes executing; there is no actual Calyx code that END_ID
/// is referring to. The NODE_ID of an invoke refers to the execution of the
/// invoke stmt itself, and the NODE_ID of an enable refers to the execution
/// of the group.
///
/// We build the domination map through the following algorithm:
/// 1) The map starts out mapping each node to the empty set.
/// 2) We run the `update_map` method, which visits each node, gets its predecessors,
/// and takes the union of the dominators of each predecessor, plus itself, and
/// updates this set as the dominators. In other words: at each node,  
/// `dom(node) = (U {dom(p) | p are all predecessors of node}) U {node}`
/// 3)Run `update_map` until the map stops changing.
///
/// The reason why we can take the union is that we know that all of the predecessors
/// of each node are guaranteed to be executed before each node. This is the reason
/// we added an "end node" to if statements, since we can think of this "end node"
/// of an if statmeent as guaranteed to be executed.
/// You may be thinking that there are actually predecessors of some nodes that
/// are not guaranteed to be executed. This happens in two cases: The
/// "end node" of if stmts and the "while guard". Let's first talk about the
/// while guard. It is true that the "while guard" has predecessor(s)-- the last
/// statement(s) in the while body-- that is not guaranteed to execute.
///
/// We can think of the while guard's predecessors in two camps: the "body predecessors"
/// that are not guaranteed to be executed before the while guard and the
/// "outside predecessors" that are outside the body of the while loop and are guaranteed
/// to be executed before the while loop guard. Since there is now a set of predecessors
/// that may *not* be executed before the while guard, you may
/// be tempted to take U(dom(outside preds)) intersect U(dom(body preds))  
/// plus the while guard itself, to get the dominators of the while guard.
/// This is indeed a correct way to calculate the dominators of the while guard.
/// However, we know that any dominators of the "outside predeecessors" must also
/// dominate "body predecessors", since in order to get to the body of a while loop
/// you must go through the while guard. Therefore, we know U dom(other preds) is a subset
/// of U dom(body preds). Therefore, taking the intersection will just yield U(dom(outside preds)).
/// Therefore, we can simply ignore the last statement(s) in the while body as predecessors
/// of the while guard when doing our calculations.
/// Similar logic applies for why we can ignore the last statment(s) of the true/false branches
/// of the if statement as predecessors of the "end node" of the if statemetn,
/// and just take the dominators of the if guard
/// plus the end node itself as the dominators of the end node of an if statement.
/// We know that every dominator of the if guard must also dominate the last statement(s)
/// of the true and false branches of the if statement. Also, we know that these
/// dominators will be the *only* common dominators of the last statement(s) of the
/// true/false branches: all the other dominators will be in either the true of false
/// branches themselves, in which case there is no overlap.

#[derive(Default)]
pub struct DominatorMap {
    /// Map from group names to the name of groups that dominate it
    pub map: HashMap<u64, HashSet<u64>>,
    /// Maps ids of control stmts, to the "last" control stmts in them. One *very*
    /// important thing to note is that this does *not* map control stmt ids to
    /// their predecessors. It maps control stmt ids to the statement that *will*
    /// be the predecessors to the stmt directly following it. For invokes and enables,
    /// this is just itself. But for seqs, for example, this will be the final invokes/enables
    /// in the seq. This is a bit confusing... so it may be wise to change the name.
    pub pred_map: HashMap<u64, HashSet<u64>>,
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
    pub fn new(control: &mut ir::Control) -> Self {
        compute_unique_ids(control, 0);
        let mut pred_map = HashMap::new();
        Self::build_predecessor_map(control, &mut pred_map);
        let mut map = DominatorMap {
            map: HashMap::new(),
            pred_map,
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
    //beginning id if begin_id is true and end_if if begin_id is false.
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

    //Builds the "predecessor map" of c. Read documentation on what this is actually
    //building. This is *not* building a map of control stmt ids to their predecessors.
    //We do that "on the fly" in the "update_map" method.
    fn build_predecessor_map(
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
                Self::build_predecessor_map(body, final_map);
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                let begin_id = Self::get_guaranteed_attribute(c, BEGIN_ID);
                let end_id = Self::get_guaranteed_attribute(c, END_ID);
                final_map.insert(begin_id, HashSet::from([end_id]));
                final_map.insert(end_id, HashSet::from([end_id]));
                Self::build_predecessor_map(tbranch, final_map);
                Self::build_predecessor_map(fbranch, final_map);
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                for stmt in stmts {
                    Self::build_predecessor_map(stmt, final_map);
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
                                .pred_map
                                .get(&prev_id)
                                .unwrap_or_else(|| {
                                    unreachable!(
                                        "pred map does not have value for {}",
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
