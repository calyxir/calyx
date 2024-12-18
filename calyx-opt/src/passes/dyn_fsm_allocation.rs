use super::math_utilities::get_bit_width_from;
use crate::passes;
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{
    self as ir, BoolAttr, GetAttributes, LibrarySignatures, Printer, RRC,
};
use calyx_ir::{build_assignments, guard, Id};
use calyx_utils::Error;
use calyx_utils::{CalyxResult, OutputFile};
use ir::Nothing;
use itertools::Itertools;
use petgraph::graph::DiGraph;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::rc::Rc;

const STATE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::STATE_ID);

const SCHEDULE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::SCHEDULE_ID);

/// Computes the exit edges of a given [ir::Control] program.
///
/// ## Example
/// In the following Calyx program:
/// ```
/// while comb_reg.out {
///   seq {
///     @STATE_ID(4) incr;
///     @STATE_ID(5) cond0;
///   }
/// }
/// ```
/// The exit edge is is `[(5, cond0[done])]` indicating that the state 5 exits when the guard
/// `cond0[done]` is true.
///
/// Multiple exit points are created when conditions are used:
/// ```
/// while comb_reg.out {
///   @STATE_ID(7) incr;
///   if comb_reg2.out {
///     @STATE_ID(8) tru;
///   } else {
///     @STATE_ID(9) fal;
///   }
/// }
/// ```
/// The exit set is `[(8, tru[done] & !comb_reg.out), (9, fal & !comb_reg.out)]`.
fn control_exits(con: &ir::Control, exits: &mut Vec<PredEdge>) {
    match con {
        ir::Control::Empty(_) => {},
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = attributes.get(STATE_ID).unwrap();
            exits.push((cur_state, guard!(group["done"])))
        },
        ir::Control::FSMEnable(ir::FSMEnable{attributes, fsm}) => {
            let cur_state = attributes.get(STATE_ID).unwrap();
            exits.push((cur_state, guard!(fsm["done"])))
        },
        ir::Control::Seq(ir::Seq { stmts, .. }) => {
            if let Some(stmt) = stmts.last() { control_exits(stmt, exits) }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            control_exits(
                tbranch, exits,
            );
            control_exits(
                fbranch, exits,
            )
        }
        ir::Control::While(ir::While { body, port, .. }) => {
            let mut loop_exits = vec![];
            control_exits(body, &mut loop_exits);
            // Loop exits only happen when the loop guard is false
            exits.extend(loop_exits.into_iter().map(|(s, g)| {
                (s, g & !ir::Guard::from(port.clone()))
            }));
        },
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Par(_) => unreachable!(),
        ir::Control::Static(_) => unreachable!(" static control should have been compiled away. Run the static compilation passes before this pass")
    }
}

/// Adds the @STATE_ID attribute to [ir::Enable] and [ir::Par].
/// Each [ir::Enable] gets a unique label within the context of a child of
/// a [ir::Par] node.
/// Furthermore, if an if/while/seq statement is labeled with a `new_fsm` attribute,
/// then it will get its own unique label. Within that if/while/seq, each enable
/// will get its own unique label within the context of that if/while/seq (see
/// example for clarification).
///
/// ## Example:
/// ```
/// seq { A; B; par { C; D; }; E; @new_fsm seq {F; G; H}}
/// ```
/// gets the labels:
/// ```
/// seq {
///   @STATE_ID(1) A; @STATE_ID(2) B;
///   @STATE_ID(3) par {
///     @STATE_ID(0) C;
///     @STATE_ID(0) D;
///   }
///   @STATE_ID(4) E;
///   @STATE_ID(5) seq{
///     @STATE_ID(0) F;
///     @STATE_ID(1) G;
///     @STATE_ID(2) H;
///   }
/// }
/// ```
///
/// These identifiers are used by the compilation methods [calculate_states_recur]
/// and [control_exits].
/// These identifiers are used by the compilation methods [calculate_states_recur]
/// and [control_exits].
fn compute_unique_state_ids(con: &mut ir::Control, cur_state: u64) -> u64 {
    match con {
      ir::Control::Enable(ir::Enable { attributes, .. }) => {
          attributes.insert(STATE_ID, cur_state);
          cur_state + 1
      }
      ir::Control::Par(ir::Par { stmts, attributes }) => {
          attributes.insert(STATE_ID, cur_state);
          stmts.iter_mut().for_each(|stmt| {
            compute_unique_state_ids(stmt, 0);
          });
          cur_state + 1
      }
      ir::Control::Seq(ir::Seq { stmts, attributes }) => {
          let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
          // if new_fsm is true, then insert attribute at the seq, and then
          // start over counting states from 0
          let mut cur = if new_fsm{
              attributes.insert(STATE_ID, cur_state);
              0
          } else {
              cur_state
          };
          stmts.iter_mut().for_each(|stmt| {
              cur = compute_unique_state_ids(stmt, cur);
          });
          // If new_fsm is true then we want to return cur_state + 1, since this
          // seq should really only take up 1 "state" on the "outer" fsm
          if new_fsm{
              cur_state + 1
          } else {
              cur
          }
      }
      ir::Control::If(ir::If {
          tbranch, fbranch, attributes, ..
      }) => {
          let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
          // if new_fsm is true, then we want to add an attribute to this
          // control statement
          if new_fsm {
              attributes.insert(STATE_ID, cur_state);
          }
          // If the program starts with a branch then branches can't get
          // the initial state.
          // Also, if new_fsm is true, we want to start with state 1 as well:
          // we can't start at 0 for the reason mentioned above
          let cur = if new_fsm || cur_state == 0 {
              1
          } else {
              cur_state
          };
          let tru_nxt = compute_unique_state_ids(
              tbranch, cur
          );
          let false_nxt = compute_unique_state_ids(
              fbranch, tru_nxt
          );
          // If new_fsm is true then we want to return cur_state + 1, since this
          // if stmt should really only take up 1 "state" on the "outer" fsm
          if new_fsm {
              cur_state + 1
          } else {
              false_nxt
          }
      }
      ir::Control::While(ir::While { body, attributes, .. }) => {
          let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
          // if new_fsm is true, then we want to add an attribute to this
          // control statement
          if new_fsm{
              attributes.insert(STATE_ID, cur_state);
          }
          // If the program starts with a branch then branches can't get
          // the initial state.
          // Also, if new_fsm is true, we want to start with state 1 as well:
          // we can't start at 0 for the reason mentioned above
          let cur = if new_fsm || cur_state == 0 {
              1
          } else {
              cur_state
          };
          let body_nxt = compute_unique_state_ids(body, cur);
          // If new_fsm is true then we want to return cur_state + 1, since this
          // while loop should really only take up 1 "state" on the "outer" fsm
          if new_fsm{
              cur_state + 1
          } else {
              body_nxt
          }
      }
      ir::Control::FSMEnable(_) => unreachable!("shouldn't encounter fsm node"),
      ir::Control::Empty(_) => cur_state,
      ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
      ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
      ir::Control::Static(_) => unreachable!("static control should have been compiled away. Run the static compilation passes before this pass")
  }
}
/// This function is used to provide a unique ID to each potential schedule within
/// a control tree. Ultimately, this ID will be used by the parent of a given schedule
/// to respectively drive and read the child's `start` and `done` wires
fn compute_unique_schedule_ids(con: &mut ir::Control, cur_sch: u64) -> u64 {
    match con {
        // no need to label enables or empty control structures; 
        // they will never get their own fsm
        ir::Control::Enable(..) | ir::Control::Empty(..) => cur_sch,

        // label a seq block; then, search for child schedules in its children
        ir::Control::Seq(ir::Seq { stmts, attributes }) => {
          let mut cur_sch = cur_sch;
          attributes.insert(SCHEDULE_ID, cur_sch);
          cur_sch += 1;
          for child in stmts.iter_mut() {
            cur_sch = compute_unique_schedule_ids(child, cur_sch);
          }
          cur_sch
        },

        // label an if; then, search for child schedules in its children
        ir::Control::If(ir::If { tbranch, fbranch, attributes, ..}) => {
          let mut cur_sch = cur_sch;
          attributes.insert(SCHEDULE_ID, cur_sch);
          cur_sch = compute_unique_schedule_ids(tbranch, cur_sch + 1);
          cur_sch = compute_unique_schedule_ids(fbranch, cur_sch);
          cur_sch
        },

        // will always allocate a par block its own fsm; 
        // then, search for child schedules in its children
        ir::Control::Par(ir::Par { stmts, attributes })  => {
          let mut cur_sch = cur_sch;
          attributes.insert(SCHEDULE_ID, cur_sch);
          cur_sch += 1;
          for child in stmts.iter_mut() {
            cur_sch = compute_unique_schedule_ids(child, cur_sch);
          }
          cur_sch
        },
        ir::Control::While(ir::While { body, attributes, .. }) => {
          let mut cur_sch = cur_sch;
          attributes.insert(SCHEDULE_ID, cur_sch);
          cur_sch = compute_unique_schedule_ids(body, cur_sch + 1);
          cur_sch
        },
        ir::Control::FSMEnable(_) => unreachable!("shouldn't encounter fsm node"),
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Static(_) => unreachable!("static control should have been compiled away. Run the static compilation passes before this pass")
    }
}

/// Represents the dyanmic execution schedule of a control program.
struct Schedule<'b, 'a: 'b> {
    /// A mapping from groups to corresponding FSM state ids
    pub groups_to_states: HashSet<FSMStateInfo>,
    /// Assigments that should be enabled in a given state.
    pub enables: HashMap<u64, Vec<ir::Assignment<Nothing>>>,
    /// FSMs that should be triggered in a given state.
    pub fsm_enables: HashMap<u64, Vec<ir::Assignment<Nothing>>>,
    /// Transition from one state to another when the guard is true.
    pub transitions: Vec<(u64, u64, ir::Guard<Nothing>)>,
    /// The component builder. The reference has a shorter lifetime than the builder itself
    /// to allow multiple schedules to use the same builder.
    pub builder: &'b mut ir::Builder<'a>,
}

/// Information to serialize for profiling purposes
#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
enum ProfilingInfo {
    Fsm(FSMInfo),
    SingleEnable(SingleEnableInfo),
}

/// Information to be serialized for a group that isn't managed by a FSM
/// This can happen if the group is the only group in a control block or a par arm
#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct SingleEnableInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub component: Id,
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub group: Id,
}

/// Information to be serialized for a single FSM
#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct FSMInfo {
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub component: Id,
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub group: Id,
    #[serde(serialize_with = "id_serialize_passthrough")]
    pub fsm: Id,
    pub states: Vec<FSMStateInfo>,
}

/// Mapping of FSM state ids to corresponding group names
#[derive(PartialEq, Eq, Hash, Clone, Serialize)]
struct FSMStateInfo {
    id: u64,
    #[serde(serialize_with = "id_serialize_passthrough")]
    group: Id,
}

fn id_serialize_passthrough<S>(id: &Id, ser: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    id.to_string().serialize(ser)
}

impl<'b, 'a> From<&'b mut ir::Builder<'a>> for Schedule<'b, 'a> {
    fn from(builder: &'b mut ir::Builder<'a>) -> Self {
        Schedule {
            groups_to_states: HashSet::new(),
            enables: HashMap::new(),
            fsm_enables: HashMap::new(),
            transitions: Vec::new(),
            builder,
        }
    }
}

impl<'b, 'a> Schedule<'b, 'a> {
    /// Validate that all states are reachable in the transition graph.
    fn validate(&self) {
        let graph = DiGraph::<(), u32>::from_edges(
            self.transitions
                .iter()
                .map(|(s, e, _)| (*s as u32, *e as u32)),
        );

        debug_assert!(
            petgraph::algo::connected_components(&graph) == 1,
            "State transition graph has unreachable states (graph has more than one connected component).");
    }

    /// Return the max state in the transition graph
    fn last_state(&self) -> u64 {
        self.transitions
            .iter()
            .max_by_key(|(_, s, _)| s)
            .expect("Schedule::transition is empty!")
            .1
    }

    /// Print out the current schedule
    fn display(&self, group: String) {
        let out = &mut std::io::stdout();
        writeln!(out, "======== {} =========", group).unwrap();
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|(state, assigns)| {
                writeln!(out, "{}:", state).unwrap();
                assigns.iter().for_each(|assign| {
                    Printer::write_assignment(assign, 2, out).unwrap();
                    writeln!(out).unwrap();
                })
            });
        writeln!(out, "{}:\n  <end>", self.last_state()).unwrap();
        writeln!(out, "transitions:").unwrap();
        self.transitions
            .iter()
            .sorted_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2))
            .for_each(|(i, f, g)| {
                writeln!(out, "  ({}, {}): {}", i, f, Printer::guard_str(g))
                    .unwrap();
            });
    }

    fn realize_fsm(self, dump_fsm: bool) -> RRC<ir::FSM> {
        // ensure schedule is valid
        self.validate();

        // compute final state and fsm_size, and register initial fsm
        let fsm = self.builder.add_fsm("fsm");

        if dump_fsm {
            self.display(format!(
                "{}:{}",
                self.builder.component.name,
                fsm.borrow().name()
            ));
        }

        // map each source state to a list of conditional transitions
        let mut transitions_map: HashMap<u64, Vec<(ir::Guard<Nothing>, u64)>> =
            HashMap::new();
        self.transitions.into_iter().for_each(
            |(s, e, g)| match transitions_map.get_mut(&(s + 1)) {
                Some(next_states) => next_states.push((g, e + 1)),
                None => {
                    transitions_map.insert(s + 1, vec![(g, e + 1)]);
                }
            },
        );

        // push the cases of the fsm to the fsm instantiation
        let (mut transitions, mut assignments): (
            VecDeque<ir::Transition>,
            VecDeque<Vec<ir::Assignment<Nothing>>>,
        ) = transitions_map
            .drain()
            .sorted_by(|(s1, _), (s2, _)| s1.cmp(s2))
            .map(|(state, mut cond_dsts)| {
                let assigns = match self.fsm_enables.get(&(state - 1)) {
                    None => vec![],
                    Some(assigns) => assigns.clone(),
                };

                // self-loop if all other guards are not met;
                // should be at the end of the conditional destinations vec!
                cond_dsts.push((ir::Guard::True, state));

                (ir::Transition::Conditional(cond_dsts), assigns)
            })
            .unzip();

        // insert transition condition from 0 to 1
        let true_guard = ir::Guard::True;
        assignments.push_front(vec![]);
        transitions.push_front(ir::Transition::Conditional(vec![
            (guard!(fsm["start"]), 1),
            (true_guard.clone(), 0),
        ]));

        // insert transition from final calc state to `done` state
        let signal_on = self.builder.add_constant(1, 1);
        let assign = build_assignments!(self.builder;
            fsm["done"] = true_guard ? signal_on["out"];
        );
        assignments.push_back(assign.to_vec());
        transitions.push_back(ir::Transition::Unconditional(0));

        fsm.borrow_mut().assignments.extend(assignments);
        fsm.borrow_mut().transitions.extend(transitions);

        // register group enables dependent on fsm state as assignments in the
        // relevant state's assignment section
        self.enables.into_iter().for_each(|(state, state_enables)| {
            fsm.borrow_mut()
                .extend_state_assignments(state + 1, state_enables);
        });
        fsm
    }
}

/// Represents an edge from a predeccesor to the current control node.
/// The `u64` represents the FSM state of the predeccesor and the guard needs
/// to be true for the predeccesor to transition to the current state.
type PredEdge = (u64, ir::Guard<Nothing>);

impl Schedule<'_, '_> {
    /// Recursively build an dynamic finite state machine represented by a [Schedule].
    /// Does the following, given an [ir::Control]:
    ///     1. If needed, add transitions from predeccesors to the current state.
    ///     2. Enable the groups in the current state
    ///     3. Calculate [PredEdge] implied by this state
    ///     4. Return [PredEdge] and the next state.
    /// Another note: the functions calc_seq_recur, calc_while_recur, and calc_if_recur
    /// are functions that `calculate_states_recur` uses for when con is a seq, while,
    /// and if respectively. The reason why they are defined as separate functions is because we
    /// need to call `calculate_seq_recur` (for example) directly when we are in `finish_seq`
    /// since `finish_seq` only gives us access to a `& mut seq` type, not a `& Control`
    /// type.
    fn calculate_states_recur(
        // Current schedule.
        &mut self,
        con: &ir::Control,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
        // True if the `@fast` attribute has successfully been applied to the parent of this control
        has_fast_guarantee: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        match con {
        ir::Control::FSMEnable(ir::FSMEnable {fsm, attributes}) => {
            let cur_state = attributes.get(STATE_ID).unwrap_or_else(|| panic!("Group `{}` does not have state_id information", fsm.borrow().name()));
            let (cur_state, prev_states) = if preds.len() == 1 && preds[0].1.is_true() {
                (preds[0].0, vec![])
            } else {
                (cur_state, preds)
            };
            // Add group to mapping for emitting group JSON info
            self.groups_to_states.insert(FSMStateInfo { id: cur_state, group: fsm.borrow().name() });

            let not_done = ir::Guard::True;
            let signal_on = self.builder.add_constant(1, 1);

            // Activate this fsm in the current state
            let en_go : [ir::Assignment<Nothing>; 1] = build_assignments!(self.builder;
                fsm["start"] = not_done ? signal_on["out"];
            );

            self.fsm_enables.entry(cur_state).or_default().extend(en_go);

            // Enable FSM to be triggered by states besides the most recent
            if early_transitions || has_fast_guarantee {
                for (st, g) in &prev_states {
                    let early_go = build_assignments!(self.builder;
                        fsm["start"] = g ? signal_on["out"];
                    );
                    self.fsm_enables.entry(*st).or_default().extend(early_go);
                }
            }

            let transitions = prev_states
                .into_iter()
                .map(|(st, guard)| (st, cur_state, guard));
            self.transitions.extend(transitions);

            let done_cond = guard!(fsm["done"]);
            Ok(vec![(cur_state, done_cond)])

        },
        // See explanation of FSM states generated in [ir::TopDownCompileControl].
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = attributes.get(STATE_ID).unwrap_or_else(|| panic!("Group `{}` does not have state_id information", group.borrow().name()));
            // If there is exactly one previous transition state with a `true`
            // guard, then merge this state into previous state.
            // This happens when the first control statement is an enable not
            // inside a branch.
            let (cur_state, prev_states) = if preds.len() == 1 && preds[0].1.is_true() {
                (preds[0].0, vec![])
            } else {
                (cur_state, preds)
            };

            // Add group to mapping for emitting group JSON info
            self.groups_to_states.insert(FSMStateInfo { id: cur_state, group: group.borrow().name() });

            let not_done = !guard!(group["done"]);
            let signal_on = self.builder.add_constant(1, 1);

            // Activate this group in the current state
            let en_go = build_assignments!(self.builder;
                group["go"] = not_done ? signal_on["out"];
            );
            self
                .enables
                .entry(cur_state)
                .or_default()
                .extend(en_go);

            // Activate group in the cycle when previous state signals done.
            // NOTE: We explicilty do not add `not_done` to the guard.
            // See explanation in [ir::TopDownCompileControl] to understand
            // why.
            if early_transitions || has_fast_guarantee {
                for (st, g) in &prev_states {
                    let early_go = build_assignments!(self.builder;
                        group["go"] = g ? signal_on["out"];
                    );
                    self.enables.entry(*st).or_default().extend(early_go);
                }
            }

            let transitions = prev_states
                .into_iter()
                .map(|(st, guard)| (st, cur_state, guard));
            self.transitions.extend(transitions);

            let done_cond = guard!(group["done"]);
            Ok(vec![(cur_state, done_cond)])
        }
        ir::Control::Seq(seq) => {
            self.calc_seq_recur(seq, preds, early_transitions)
        }
        ir::Control::If(if_stmt) => {
            self.calc_if_recur(if_stmt, preds, early_transitions)
        }
        ir::Control::While(while_stmt) => {
            self.calc_while_recur(while_stmt, preds, early_transitions)
        }
        ir::Control::Par(_) => unreachable!(),
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Empty(_) => unreachable!("`calculate_states_recur` should not see an `empty` control."),
        ir::Control::Static(_) => unreachable!("static control should have been compiled away. Run the static compilation passes before this pass")
    }
    }

    /// Builds a finite state machine for `seq` represented by a [Schedule].
    /// At a high level, it iterates through each stmt in the seq's control, using the
    /// previous stmt's [PredEdge] as the `preds` for the current stmt, and returns
    /// the [PredEdge] implied by the last stmt in `seq`'s control.
    fn calc_seq_recur(
        &mut self,
        seq: &ir::Seq,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        let mut prev = preds;
        for (i, stmt) in seq.stmts.iter().enumerate() {
            prev = self.calculate_states_recur(
                stmt,
                prev,
                early_transitions,
                i > 0 && seq.get_attributes().has(BoolAttr::Fast),
            )?;
        }
        Ok(prev)
    }

    /// Builds a finite state machine for `if_stmt` represented by a [Schedule].
    /// First generates the transitions into the true branch + the transitions that exist
    /// inside the true branch. Then generates the transitions into the false branch + the transitions
    /// that exist inside the false branch. Then calculates the transitions needed to
    /// exit the if statmement (which include edges from both the true and false branches).
    fn calc_if_recur(
        &mut self,
        if_stmt: &ir::If,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        if if_stmt.cond.is_some() {
            return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", DynamicFSMAllocation::name(), if_stmt.cond.as_ref().unwrap().borrow().name())));
        }
        let port_guard: ir::Guard<Nothing> = Rc::clone(&if_stmt.port).into();
        // Previous states transitioning into true branch need the conditional
        // to be true.
        let tru_transitions = preds
            .clone()
            .into_iter()
            .map(|(s, g)| (s, g & port_guard.clone()))
            .collect();
        let tru_prev = self.calculate_states_recur(
            &if_stmt.tbranch,
            tru_transitions,
            early_transitions,
            false,
        )?;
        // Previous states transitioning into false branch need the conditional
        // to be false.
        let fal_transitions = preds
            .into_iter()
            .map(|(s, g)| (s, g & !port_guard.clone()))
            .collect();

        let fal_prev = if let ir::Control::Empty(..) = *if_stmt.fbranch {
            // If the false branch is empty, then all the prevs to this node will become prevs
            // to the next node.
            fal_transitions
        } else {
            self.calculate_states_recur(
                &if_stmt.fbranch,
                fal_transitions,
                early_transitions,
                false,
            )?
        };

        let prevs = tru_prev.into_iter().chain(fal_prev).collect();
        Ok(prevs)
    }

    /// Builds a finite state machine for `while_stmt` represented by a [Schedule].
    /// It first generates the backwards edges (i.e., edges from the end of the while
    /// body back to the beginning of the while body), then generates the forwards
    /// edges in the body, then generates the edges that exit the while loop.
    fn calc_while_recur(
        &mut self,
        while_stmt: &ir::While,
        // The set of previous states that want to transition into cur_state
        preds: Vec<PredEdge>,
        // True if early_transitions are allowed
        early_transitions: bool,
    ) -> CalyxResult<Vec<PredEdge>> {
        if while_stmt.cond.is_some() {
            return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", DynamicFSMAllocation::name(), while_stmt.cond.as_ref().unwrap().borrow().name())));
        }

        let port_guard: ir::Guard<Nothing> = Rc::clone(&while_stmt.port).into();

        // Step 1: Generate the backward edges by computing the exit nodes.
        let mut exits = vec![];
        control_exits(&while_stmt.body, &mut exits);

        // Step 2: Generate the forward edges normally.
        // Previous transitions into the body require the condition to be
        // true.
        let transitions: Vec<PredEdge> = preds
            .clone()
            .into_iter()
            .chain(exits)
            .map(|(s, g)| (s, g & port_guard.clone()))
            .collect();
        let prevs = self.calculate_states_recur(
            &while_stmt.body,
            transitions,
            early_transitions,
            false,
        )?;

        // Step 3: The final out edges from the while come from:
        //   - Before the body when the condition is false
        //   - Inside the body when the condition is false
        let not_port_guard = !port_guard;
        let all_prevs = preds
            .into_iter()
            .chain(prevs)
            .map(|(st, guard)| (st, guard & not_port_guard.clone()))
            .collect();

        Ok(all_prevs)
    }

    /// Creates a Schedule that represents `seq`, mainly relying on `calc_seq_recur()`.
    fn calculate_states_seq(
        &mut self,
        seq: &ir::Seq,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev =
            self.calc_seq_recur(seq, vec![first_state], early_transitions)?;
        self.add_nxt_transition(prev);
        Ok(())
    }

    /// Creates a Schedule that represents `if`, mainly relying on `calc_if_recur()`.
    fn calculate_states_if(
        &mut self,
        if_stmt: &ir::If,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev =
            self.calc_if_recur(if_stmt, vec![first_state], early_transitions)?;
        self.add_nxt_transition(prev);
        Ok(())
    }

    /// Creates a Schedule that represents `while`, mainly relying on `calc_while_recur()`.
    fn calculate_states_while(
        &mut self,
        while_stmt: &ir::While,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev = self.calc_while_recur(
            while_stmt,
            vec![first_state],
            early_transitions,
        )?;
        self.add_nxt_transition(prev);
        Ok(())
    }

    /// Given predecessors prev, creates a new "next" state and transitions from
    /// each state in prev to the next state.
    /// In other words, it just adds an "end" state to [Schedule] and the
    /// appropriate transitions to that "end" state.
    fn add_nxt_transition(&mut self, prev: Vec<PredEdge>) {
        let nxt = prev
            .iter()
            .max_by(|(st1, _), (st2, _)| st1.cmp(st2))
            .unwrap()
            .0
            + 1;
        let transitions = prev.into_iter().map(|(st, guard)| (st, nxt, guard));
        self.transitions.extend(transitions);
    }

    /// Note: the functions calculate_states_seq, calculate_states_while, and calculate_states_if
    /// are functions that basically do what `calculate_states` would do if `calculate_states` knew (for certain)
    /// that its input parameter would be a seq/while/if.
    /// The reason why we need to define these as separate functions is because `finish_seq`
    /// (for example) we only gives us access to a `& mut seq` type, not a `& Control`
    /// type.
    fn calculate_states(
        &mut self,
        con: &ir::Control,
        early_transitions: bool,
    ) -> CalyxResult<()> {
        let first_state = (0, ir::Guard::True);
        // We create an empty first state in case the control program starts with
        // a branch (if, while).
        // If the program doesn't branch, then the initial state is merged into
        // the first group.
        let prev = self.calculate_states_recur(
            con,
            vec![first_state],
            early_transitions,
            false,
        )?;
        self.add_nxt_transition(prev);
        Ok(())
    }
}

/// **Core lowering pass.**
/// Compiles away the control programs in components into purely structural code using an
/// finite-state machine (FSM).
///
/// Lowering operates in two steps:
/// 1. Compile all [ir::Par] control sub-programs into a single [ir::Enable] of a group that runs
///    all children to completion.
/// 2. Compile the top-level control program into a single [ir::Enable].
///
/// ## Compiling non-`par` programs
/// At very high-level, the pass assigns an FSM state to each [ir::Enable] in the program and
/// generates transitions to the state to activate the groups contained within the [ir::Enable].
///
/// The compilation process calculates all predeccesors of the [ir::Enable] while walking over the
/// control program. A predeccesor is any enable statement that can directly "jump" to the current
/// [ir::Enable]. The compilation process computes all such predeccesors and the guards that need
/// to be true for the predeccesor to jump into this enable statement.
///
/// ```
/// cond0;
/// while lt.out {
///   if gt.out { true } else { false }
/// }
/// next;
/// ```
/// The predeccesor sets are:
/// ```
/// cond0 -> []
/// true -> [(cond0, lt.out & gt.out); (true; lt.out & gt.out); (false, lt.out & !gt.out)]
/// false -> [(cond0, lt.out & !gt.out); (true; lt.out & gt.out); (false, lt.out & !gt.out)]
/// next -> [(cond0, !lt.out); (true, !lt.out); (false, !lt.out)]
/// ```
///
/// ### Compiling [ir::Enable]
/// The process first takes all edges from predeccesors and transitions to the state for this
/// enable and enables the group in this state:
/// ```text
/// let cur_state; // state of this enable
/// for (state, guard) in predeccesors:
///   transitions.insert(state, cur_state, guard)
/// enables.insert(cur_state, group)
/// ```
///
/// While this process will generate a functioning FSM, the FSM takes unnecessary cycles for FSM
/// transitions.
///
/// For example:
/// ```
/// seq { one; two; }
/// ```
/// The FSM generated will look like this (where `f` is the FSM register):
/// ```
/// f.in = one[done] ? 1;
/// f.in = two[done] ? 2;
/// one[go] = !one[done] & f.out == 0;
/// two[go] = !two[done] & f.out == 1;
/// ```
///
/// The cycle-level timing for this FSM will look like:
///     - cycle 0: (`f.out` == 0), enable one
///     - cycle t: (`f.out` == 0), (`one[done]` == 1), disable one
///     - cycle t+1: (`f.out` == 1), enable two
///     - cycle t+l: (`f.out` == 1), (`two[done]` == 1), disable two
///     - cycle t+l+1: finish
///
/// The transition t -> t+1 represents one where group one is done but group two hasn't started
/// executing.
///
/// To address this specific problem, there is an additional enable added to run all groups within
/// an enable *while the FSM is transitioning*.
/// The final transition will look like this:
/// ```
/// f.in = one[done] ? 1;
/// f.in = two[done] ? 2;
/// one[go] = !one[done] & f.out == 0;
/// two[go] = (!two[done] & f.out == 1) || (one[done] & f.out == 0);
/// ```
///
/// Note that `!two[done]` isn't present in the second disjunct because all groups are guaranteed
/// to run for at least one cycle and the second disjunct will only be true for one cycle before
/// the first disjunct becomes true.
///
/// ## Compiling `par` programs
/// We have to generate new FSM-based controller for each child of a `par` node so that each child
/// can indepdendently make progress.
/// If we tie the children to one top-level FSM, their transitions would become interdependent and
/// reduce available concurrency.
///
/// ## Compilation guarantee
/// At the end of this pass, the control program will have no more than one
/// group enable in it.
pub struct DynamicFSMAllocation {
    /// Print out the FSM representation to STDOUT
    dump_fsm: bool,
    /// Output a JSON FSM representation to file if specified
    dump_fsm_json: Option<OutputFile>,
    /// Enable early transitions
    early_transitions: bool,
    /// Bookkeeping for FSM ids for groups across all FSMs in the program
    fsm_groups: HashSet<ProfilingInfo>,
}

impl ConstructVisitor for DynamicFSMAllocation {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        Ok(DynamicFSMAllocation {
            dump_fsm: opts[&"dump-fsm"].bool(),
            dump_fsm_json: opts[&"dump-fsm-json"].not_null_outstream(),
            early_transitions: opts[&"early-transitions"].bool(),
            fsm_groups: HashSet::new(),
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Named for DynamicFSMAllocation {
    fn name() -> &'static str {
        "dfsm"
    }

    fn description() -> &'static str {
        "Removing control constructs and instantiate FSMs in their place"
    }

    fn opts() -> Vec<PassOpt> {
        vec![
            PassOpt::new(
                "dump-fsm",
                "Print out the state machine implementing the schedule",
                ParseVal::Bool(false),
                PassOpt::parse_bool,
            ),
            PassOpt::new(
                "dump-fsm-json",
                "Write the state machine implementing the schedule to a JSON file",
                ParseVal::OutStream(OutputFile::Null),
                PassOpt::parse_outstream,
            ),
            PassOpt::new(
                "early-transitions",
                "Experimental: Enable early transitions for group enables",
                ParseVal::Bool(false),
                PassOpt::parse_bool,
            ),
        ]
    }
}

/// Helper function to emit profiling information when the control consists of a single group.
fn extract_single_enable(
    con: &mut ir::Control,
    component: Id,
) -> Option<SingleEnableInfo> {
    if let ir::Control::Enable(enable) = con {
        return Some(SingleEnableInfo {
            component,
            group: enable.group.borrow().name(),
        });
    } else {
        None
    }
}

impl Visitor for DynamicFSMAllocation {
    fn start(
        &mut self,
        comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let mut con = comp.control.borrow_mut();
        if matches!(*con, ir::Control::Empty(..) | ir::Control::Enable(..)) {
            if let Some(enable_info) =
                extract_single_enable(&mut con, comp.name)
            {
                self.fsm_groups
                    .insert(ProfilingInfo::SingleEnable(enable_info));
            }
            return Ok(Action::Stop);
        }

        compute_unique_state_ids(&mut con, 0);
        compute_unique_schedule_ids(&mut con, 0);
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut calyx_ir::Seq,
        comp: &mut calyx_ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        if !s.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }

        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);
        sch.calculate_states_seq(s, self.early_transitions)?;
        let seq_fsm = sch.realize_fsm(self.dump_fsm);
        let mut fsm_en = ir::Control::fsm_enable(seq_fsm);
        let state_id = s.attributes.get(STATE_ID).unwrap();
        fsm_en.get_mut_attributes().insert(STATE_ID, state_id);

        Ok(Action::change(fsm_en))
    }

    fn finish_par(
        &mut self,
        _s: &mut calyx_ir::Par,
        comp: &mut calyx_ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        // let par_fsm
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let control = Rc::clone(&comp.control);

        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);

        // Add assignments for the final states
        sch.calculate_states(&control.borrow(), self.early_transitions)?;
        let comp_fsm = sch.realize_fsm(self.dump_fsm);

        Ok(Action::change(ir::Control::fsm_enable(comp_fsm)))
    }
}
