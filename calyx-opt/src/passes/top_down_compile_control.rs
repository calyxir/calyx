use super::math_utilities::get_bit_width_from;
use crate::passes;
use crate::traversal::{
    Action, ConstructVisitor, Named, ParseVal, PassOpt, VisResult, Visitor,
};
use calyx_ir::{
    self as ir, BoolAttr, Cell, GetAttributes, LibrarySignatures, Printer, RRC,
};
use calyx_ir::{build_assignments, guard, structure, Id};
use calyx_utils::Error;
use calyx_utils::{CalyxResult, OutputFile};
use ir::Nothing;
use itertools::Itertools;
use petgraph::graph::DiGraph;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::rc::Rc;

const NODE_ID: ir::Attribute =
    ir::Attribute::Internal(ir::InternalAttr::NODE_ID);
const DUPLICATE_NUM_REG: u64 = 2;

/// Computes the exit edges of a given [ir::Control] program.
///
/// ## Example
/// In the following Calyx program:
/// ```
/// while comb_reg.out {
///   seq {
///     @NODE_ID(4) incr;
///     @NODE_ID(5) cond0;
///   }
/// }
/// ```
/// The exit edge is is `[(5, cond0[done])]` indicating that the state 5 exits when the guard
/// `cond0[done]` is true.
///
/// Multiple exit points are created when conditions are used:
/// ```
/// while comb_reg.out {
///   @NODE_ID(7) incr;
///   if comb_reg2.out {
///     @NODE_ID(8) tru;
///   } else {
///     @NODE_ID(9) fal;
///   }
/// }
/// ```
/// The exit set is `[(8, tru[done] & !comb_reg.out), (9, fal & !comb_reg.out)]`.
fn control_exits(con: &ir::Control, exits: &mut Vec<PredEdge>) {
    match con {
        ir::Control::Empty(_) => {}
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = attributes.get(NODE_ID).unwrap();
            exits.push((cur_state, guard!(group["done"])))
        }
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

/// Adds the @NODE_ID attribute to [ir::Enable] and [ir::Par].
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
///   @NODE_ID(1) A; @NODE_ID(2) B;
///   @NODE_ID(3) par {
///     @NODE_ID(0) C;
///     @NODE_ID(0) D;
///   }
///   @NODE_ID(4) E;
///   @NODE_ID(5) seq{
///     @NODE_ID(0) F;
///     @NODE_ID(1) G;
///     @NODE_ID(2) H;
///   }
/// }
/// ```
///
/// These identifiers are used by the compilation methods [calculate_states_recur]
/// and [control_exits].
fn compute_unique_ids(con: &mut ir::Control, cur_state: u64) -> u64 {
    match con {
        ir::Control::Enable(ir::Enable { attributes, .. }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state + 1
        }
        ir::Control::Par(ir::Par { stmts, attributes }) => {
            attributes.insert(NODE_ID, cur_state);
            stmts.iter_mut().for_each(|stmt| {
                compute_unique_ids(stmt, 0);
            });
            cur_state + 1
        }
        ir::Control::Seq(ir::Seq { stmts, attributes }) => {
            let new_fsm = attributes.has(ir::BoolAttr::NewFSM);
            // if new_fsm is true, then insert attribute at the seq, and then
            // start over counting states from 0
            let mut cur = if new_fsm{
                attributes.insert(NODE_ID, cur_state);
                0
            } else {
                cur_state
            };
            stmts.iter_mut().for_each(|stmt| {
                cur = compute_unique_ids(stmt, cur);
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
                attributes.insert(NODE_ID, cur_state);
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
            let tru_nxt = compute_unique_ids(
                tbranch, cur
            );
            let false_nxt = compute_unique_ids(
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
                attributes.insert(NODE_ID, cur_state);
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
            let body_nxt = compute_unique_ids(body, cur);
            // If new_fsm is true then we want to return cur_state + 1, since this
            // while loop should really only take up 1 "state" on the "outer" fsm
            if new_fsm{
                cur_state + 1
            } else {
                body_nxt
            }
        }
        ir::Control::Empty(_) => cur_state,
        ir::Control::Repeat(_) => unreachable!("`repeat` statements should have been compiled away. Run `{}` before this pass.", passes::CompileRepeat::name()),
        ir::Control::Invoke(_) => unreachable!("`invoke` statements should have been compiled away. Run `{}` before this pass.", passes::CompileInvoke::name()),
        ir::Control::Static(_) => unreachable!("static control should have been compiled away. Run the static compilation passes before this pass")
    }
}

/// Given the state of the FSM, returns the index for the register in `fsms``
/// that should be queried.
/// A query for each state must read from one of the `num_registers` registers.
/// For `r` registers and `n` states, we split into "buckets" as follows:
/// `{0, ... , n/r - 1} -> reg. @ index 0`,
/// `{n/r, ... , 2n/r - 1} -> reg. @ index 1`,
/// ...,
/// `{(r-1)n/r, ... , n - 1} -> reg. @ index n - 1`.
/// Note that dividing each state by the value `n/r`normalizes the state w.r.t.
/// the FSM register from which it should read. We can then take the floor of this value
/// (or, equivalently, use unsigned integer division) to get this register index.
fn register_to_query(
    state: u64,
    num_states: u64,
    num_registers: u64,
    distribute: bool,
) -> usize {
    match distribute {
        true => {
            // num_states+1 is needed to prevent error (the done condition needs
            // to check past the number of states, i.e., will check fsm == 3 when
            // num_states == 3).
            (state * num_registers / (num_states + 1))
                .try_into()
                .unwrap()
        }
        false => 0,
    }
}

#[derive(Clone, Copy)]
enum RegisterEncoding {
    Binary,
    OneHot,
}
#[derive(Clone, Copy)]
enum RegisterSpread {
    // Default option: just a single register
    Single,
    // Duplicate the register to reduce fanout when querying
    // (all FSMs in this vec still have all of the states)
    Duplicate,
}

#[derive(Clone, Copy)]
/// A type that represents how the FSM should be implemented in hardware.
struct FSMRepresentation {
    // the representation of a state within a register (one-hot, binary, etc.)
    encoding: RegisterEncoding,
    // the number of registers representing the dynamic finite state machine
    spread: RegisterSpread,
    // the index of the last state in the fsm (total # states = last_state + 1)
    last_state: u64,
}

/// Represents the dyanmic execution schedule of a control program.
struct Schedule<'b, 'a: 'b> {
    /// A mapping from groups to corresponding FSM state ids
    pub groups_to_states: HashSet<FSMStateInfo>,
    /// Assigments that should be enabled in a given state.
    pub enables: HashMap<u64, Vec<ir::Assignment<Nothing>>>,
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

    /// First chooses which register to query from (only relevant in the duplication case.)
    /// Then queries the FSM by building a new slicer and corresponding assignments if
    /// the query hasn't yet been made. If this query has been made before with one-hot
    /// encoding, it reuses the old query, but always returns a new guard representing the query.
    fn query_state(
        builder: &mut ir::Builder,
        used_slicers_vec: &mut [HashMap<u64, RRC<Cell>>],
        fsm_rep: &FSMRepresentation,
        hardware: (&[RRC<ir::Cell>], &RRC<Cell>),
        state: &u64,
        fsm_size: &u64,
        distribute: bool,
    ) -> ir::Guard<Nothing> {
        let (fsms, signal_on) = hardware;
        let (fsm, used_slicers) = {
            let reg_to_query = register_to_query(
                *state,
                fsm_rep.last_state,
                fsms.len().try_into().unwrap(),
                distribute,
            );
            (
                fsms.get(reg_to_query)
                    .expect("the register at this index does not exist"),
                used_slicers_vec
                    .get_mut(reg_to_query)
                    .expect("the used slicer map at this index does not exist"),
            )
        };
        match fsm_rep.encoding {
            RegisterEncoding::Binary => {
                let state_const = builder.add_constant(*state, *fsm_size);
                let state_guard = guard!(fsm["out"] == state_const["out"]);
                state_guard
            }
            RegisterEncoding::OneHot => {
                match used_slicers.get(state) {
                    None => {
                        // construct slicer for this bit query
                        structure!(
                            builder;
                            let slicer = prim std_bit_slice(*fsm_size, *state, *state, 1);
                        );
                        // build wire from fsm to slicer
                        let fsm_to_slicer = builder.build_assignment(
                            slicer.borrow().get("in"),
                            fsm.borrow().get("out"),
                            ir::Guard::True,
                        );
                        // add continuous assignments to slicer
                        builder
                            .component
                            .continuous_assignments
                            .push(fsm_to_slicer);
                        // create a guard representing when to allow next-state transition
                        let state_guard =
                            guard!(slicer["out"] == signal_on["out"]);
                        used_slicers.insert(*state, slicer);
                        state_guard
                    }
                    Some(slicer) => {
                        let state_guard =
                            guard!(slicer["out"] == signal_on["out"]);
                        state_guard
                    }
                }
            }
        }
    }

    /// Builds the register(s) and constants needed for a given encoding and spread type.
    fn build_fsm_infrastructure(
        builder: &mut ir::Builder,
        fsm_rep: &FSMRepresentation,
    ) -> (Vec<RRC<Cell>>, RRC<Cell>, u64) {
        // get fsm bit width and build constant emitting fsm first state
        let (fsm_size, first_state) = match fsm_rep.encoding {
            RegisterEncoding::Binary => {
                let fsm_size = get_bit_width_from(fsm_rep.last_state + 1);
                (fsm_size, builder.add_constant(0, fsm_size))
            }
            RegisterEncoding::OneHot => {
                let fsm_size = fsm_rep.last_state + 1;
                (fsm_size, builder.add_constant(1, fsm_size))
            }
        };

        // for the given number of fsm registers to read from, add a primitive register to the design for each
        let mut add_fsm_regs = |prim_name: &str, num_regs: u64| {
            (0..num_regs)
                .map(|n| {
                    let fsm_name = if num_regs == 1 {
                        "fsm".to_string()
                    } else {
                        format!("fsm{}", n)
                    };
                    builder.add_primitive(
                        fsm_name.as_str(),
                        prim_name,
                        &[fsm_size],
                    )
                })
                .collect_vec()
        };

        let fsms = match (fsm_rep.encoding, fsm_rep.spread) {
            (RegisterEncoding::Binary, RegisterSpread::Single) => {
                add_fsm_regs("std_reg", 1)
            }
            (RegisterEncoding::OneHot, RegisterSpread::Single) => {
                add_fsm_regs("init_one_reg", 1)
            }
            (RegisterEncoding::Binary, RegisterSpread::Duplicate) => {
                add_fsm_regs("std_reg", DUPLICATE_NUM_REG)
            }
            (RegisterEncoding::OneHot, RegisterSpread::Duplicate) => {
                add_fsm_regs("init_one_reg", DUPLICATE_NUM_REG)
            }
        };

        (fsms, first_state, fsm_size)
    }

    /// Implement a given [Schedule] and return the name of the [ir::Group] that
    /// implements it.
    fn realize_schedule(
        self,
        dump_fsm: bool,
        fsm_groups: &mut HashSet<ProfilingInfo>,
        fsm_rep: FSMRepresentation,
    ) -> RRC<ir::Group> {
        // confirm all states are reachable
        self.validate();

        // build tdcc group
        let group = self.builder.add_group("tdcc");
        if dump_fsm {
            self.display(format!(
                "{}:{}",
                self.builder.component.name,
                group.borrow().name()
            ));
        }

        // build necessary primitives dependent on encoding and spread
        let signal_on = self.builder.add_constant(1, 1);
        let (fsms, first_state, fsm_size) =
            Self::build_fsm_infrastructure(self.builder, &fsm_rep);

        // get first fsm register
        let fsm1 = fsms.first().expect("first fsm register does not exist");

        // Add last state to JSON info
        let mut states = self.groups_to_states.iter().cloned().collect_vec();
        states.push(FSMStateInfo {
            id: fsm_rep.last_state, // check that this register (fsm.0) is the correct one to use
            group: Id::new(format!("{}_END", fsm1.borrow().name())),
        });

        // Keep track of groups to FSM state id information for dumping to json
        fsm_groups.insert(ProfilingInfo::Fsm(FSMInfo {
            component: self.builder.component.name,
            fsm: fsm1.borrow().name(),
            group: group.borrow().name(),
            states,
        }));

        // keep track of used slicers if using one hot encoding. one for each register
        let mut used_slicers_vec =
            fsms.iter().map(|_| HashMap::new()).collect_vec();

        // enable assignments
        // the following enable queries; we can decide which register to query for state-dependent assignments
        // because we know all registers precisely agree at each cycle
        group.borrow_mut().assignments.extend(
            self.enables
                .into_iter()
                .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                .flat_map(|(state, mut assigns)| {
                    // for every assignment dependent on current fsm state, `&` new guard with existing guard
                    let state_guard = Self::query_state(
                        self.builder,
                        &mut used_slicers_vec,
                        &fsm_rep,
                        (&fsms, &signal_on),
                        &state,
                        &fsm_size,
                        true, // by default attempt to distribute across regs if >=2 exist
                    );
                    assigns.iter_mut().for_each(|asgn| {
                        asgn.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        // transition assignments
        // the following updates are meant to ensure agreement between the two
        // fsm registers; hence, all registers must be updated if `duplicate` is chosen
        group.borrow_mut().assignments.extend(
            self.transitions.into_iter().flat_map(|(s, e, guard)| {
                // get a transition guard for the first fsm register, and apply it to every fsm register
                let state_guard = Self::query_state(
                    self.builder,
                    &mut used_slicers_vec,
                    &fsm_rep,
                    (&fsms, &signal_on),
                    &s,
                    &fsm_size,
                    false, // by default do not distribute transition queries across regs; choose first
                );

                // add transitions for every fsm register to ensure consistency between each
                fsms.iter()
                    .flat_map(|fsm| {
                        let trans_guard =
                            state_guard.clone().and(guard.clone());
                        let end_const = match fsm_rep.encoding {
                            RegisterEncoding::Binary => {
                                self.builder.add_constant(e, fsm_size)
                            }
                            RegisterEncoding::OneHot => {
                                self.builder.add_constant(
                                    u64::pow(
                                        2,
                                        e.try_into()
                                            .expect("failed to convert to u32"),
                                    ),
                                    fsm_size,
                                )
                            }
                        };
                        let ec_borrow = end_const.borrow();
                        vec![
                            self.builder.build_assignment(
                                fsm.borrow().get("in"),
                                ec_borrow.get("out"),
                                trans_guard.clone(),
                            ),
                            self.builder.build_assignment(
                                fsm.borrow().get("write_en"),
                                signal_on.borrow().get("out"),
                                trans_guard,
                            ),
                        ]
                    })
                    .collect_vec()
            }),
        );

        // done condition for group
        // arbitrarily look at first fsm register, since all are identical
        let first_fsm_last_guard = Self::query_state(
            self.builder,
            &mut used_slicers_vec,
            &fsm_rep,
            (&fsms, &signal_on),
            &fsm_rep.last_state,
            &fsm_size,
            false,
        );

        let done_assign = self.builder.build_assignment(
            group.borrow().get("done"),
            signal_on.borrow().get("out"),
            first_fsm_last_guard.clone(),
        );

        group.borrow_mut().assignments.push(done_assign);

        // Cleanup: Add a transition from last state to the first state for each register
        let reset_fsms = fsms
            .iter()
            .flat_map(|fsm| {
                // by default, query first register
                let fsm_last_guard = Self::query_state(
                    self.builder,
                    &mut used_slicers_vec,
                    &fsm_rep,
                    (&fsms, &signal_on),
                    &fsm_rep.last_state,
                    &fsm_size,
                    false,
                );
                let reset_fsm = build_assignments!(self.builder;
                    fsm["in"] = fsm_last_guard ? first_state["out"];
                    fsm["write_en"] = fsm_last_guard ? signal_on["out"];
                );
                reset_fsm.to_vec()
            })
            .collect_vec();

        // extend with conditions to set all fsms to initial state
        self.builder
            .component
            .continuous_assignments
            .extend(reset_fsms);

        group
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
        // See explanation of FSM states generated in [ir::TopDownCompileControl].
        ir::Control::Enable(ir::Enable { group, attributes }) => {
            let cur_state = attributes.get(NODE_ID).unwrap_or_else(|| panic!("Group `{}` does not have node_id information", group.borrow().name()));
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
            return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownCompileControl::name(), if_stmt.cond.as_ref().unwrap().borrow().name())));
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
            return Err(Error::malformed_structure(format!("{}: Found group `{}` in with position of if. This should have compiled away.", TopDownCompileControl::name(), while_stmt.cond.as_ref().unwrap().borrow().name())));
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
pub struct TopDownCompileControl {
    /// Print out the FSM representation to STDOUT
    dump_fsm: bool,
    /// Output a JSON FSM representation to file if specified
    dump_fsm_json: Option<OutputFile>,
    /// Enable early transitions
    early_transitions: bool,
    /// Bookkeeping for FSM ids for groups across all FSMs in the program
    fsm_groups: HashSet<ProfilingInfo>,
    /// How many states the dynamic FSM must have before picking binary over one-hot
    one_hot_cutoff: u64,
    /// Number of states the dynamic FSM must have before picking duplicate over single register
    duplicate_cutoff: u64,
}

impl TopDownCompileControl {
    /// Given a dynamic schedule and attributes, selects a representation for
    /// the finite state machine in hardware.
    fn get_representation(
        &self,
        sch: &Schedule,
        attrs: &ir::Attributes,
    ) -> FSMRepresentation {
        let last_state = sch.last_state();
        FSMRepresentation {
            encoding: {
                match (
                    attrs.has(BoolAttr::OneHot),
                    last_state <= self.one_hot_cutoff,
                ) {
                    (true, _) | (false, true) => RegisterEncoding::OneHot,
                    (false, false) => RegisterEncoding::Binary,
                }
            },
            spread: {
                match (last_state + 1) <= self.duplicate_cutoff {
                    true => RegisterSpread::Single,
                    false => RegisterSpread::Duplicate,
                }
            },
            last_state,
        }
    }
}

impl ConstructVisitor for TopDownCompileControl {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(ctx);

        Ok(TopDownCompileControl {
            dump_fsm: opts[&"dump-fsm"].bool(),
            dump_fsm_json: opts[&"dump-fsm-json"].not_null_outstream(),
            early_transitions: opts[&"early-transitions"].bool(),
            fsm_groups: HashSet::new(),
            one_hot_cutoff: opts[&"one-hot-cutoff"]
                .pos_num()
                .expect("requires non-negative OHE cutoff parameter"),
            duplicate_cutoff: opts[&"duplicate-cutoff"]
                .pos_num()
                .expect("requires non-negative duplicate cutoff parameter"),
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Named for TopDownCompileControl {
    fn name() -> &'static str {
        "tdcc"
    }

    fn description() -> &'static str {
        "Top-down compilation for removing control constructs"
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
            PassOpt::new(
                "one-hot-cutoff",
                "Threshold at and below which a one-hot encoding is used for dynamic group scheduling",
                ParseVal::Num(0),
                PassOpt::parse_num,
            ),
            PassOpt::new(
                "duplicate-cutoff",
                "Threshold above which the dynamic fsm register is replicated into a second, identical register",
                ParseVal::Num(i64::MAX),
                PassOpt::parse_num,
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

impl Visitor for TopDownCompileControl {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
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

        compute_unique_ids(&mut con, 0);
        // IRPrinter::write_control(&con, 0, &mut std::io::stderr());
        Ok(Action::Continue)
    }

    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // only compile using new fsm if has new_fsm attribute
        if !s.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);
        sch.calculate_states_seq(s, self.early_transitions)?;
        let fsm_impl = self.get_representation(&sch, &s.attributes);

        // Compile schedule and return the group.
        let seq_group =
            sch.realize_schedule(self.dump_fsm, &mut self.fsm_groups, fsm_impl);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(seq_group);
        let node_id = s.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    fn finish_if(
        &mut self,
        i: &mut ir::If,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // only compile using new fsm if has new_fsm attribute
        if !i.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);

        // Compile schedule and return the group.
        sch.calculate_states_if(i, self.early_transitions)?;
        let fsm_impl = self.get_representation(&sch, &i.attributes);
        let if_group =
            sch.realize_schedule(self.dump_fsm, &mut self.fsm_groups, fsm_impl);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(if_group);
        let node_id = i.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    fn finish_while(
        &mut self,
        w: &mut ir::While,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // only compile using new fsm if has attribute
        if !w.attributes.has(ir::BoolAttr::NewFSM) {
            return Ok(Action::Continue);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);
        sch.calculate_states_while(w, self.early_transitions)?;
        let fsm_impl = self.get_representation(&sch, &w.attributes);

        // Compile schedule and return the group.
        let if_group =
            sch.realize_schedule(self.dump_fsm, &mut self.fsm_groups, fsm_impl);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(if_group);
        let node_id = w.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    /// Compile each child in `par` block separately so each child can make
    /// progress indepdendently.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);

        // Compilation group
        let par_group = builder.add_group("par");
        structure!(builder;
            let signal_on = constant(1, 1);
            let signal_off = constant(0, 1);
        );

        // Registers to save the done signal from each child.
        let mut done_regs = Vec::with_capacity(s.stmts.len());

        // For each child, build the enabling logic.
        for con in &s.stmts {
            let group = match con {
                // Do not compile enables
                ir::Control::Enable(ir::Enable { group, .. }) => {
                    self.fsm_groups.insert(ProfilingInfo::SingleEnable(
                        SingleEnableInfo {
                            group: group.borrow().name(),
                            component: builder.component.name,
                        },
                    ));
                    Rc::clone(group)
                }
                // Compile complex schedule and return the group.
                _ => {
                    let mut sch = Schedule::from(&mut builder);
                    sch.calculate_states(con, self.early_transitions)?;
                    let fsm_impl = self.get_representation(&sch, &s.attributes);
                    sch.realize_schedule(
                        self.dump_fsm,
                        &mut self.fsm_groups,
                        fsm_impl,
                    )
                }
            };

            // Build circuitry to enable and disable this group.
            structure!(builder;
                let pd = prim std_reg(1);
            );
            let group_go = !(guard!(pd["out"] | group["done"]));
            let group_done = guard!(group["done"]);

            // Save the done condition in a register.
            let assigns = build_assignments!(builder;
                group["go"] = group_go ? signal_on["out"];
                pd["in"] = group_done ? signal_on["out"];
                pd["write_en"] = group_done ? signal_on["out"];
            );
            par_group.borrow_mut().assignments.extend(assigns);
            done_regs.push(pd)
        }

        // Done condition for this group
        let done_guard = done_regs
            .clone()
            .into_iter()
            .map(|r| guard!(r["out"]))
            .fold(ir::Guard::True, ir::Guard::and);

        // CLEANUP: Reset the registers once the group is finished.
        let mut cleanup = done_regs
            .into_iter()
            .flat_map(|r| {
                build_assignments!(builder;
                    r["in"] = done_guard ? signal_off["out"];
                    r["write_en"] = done_guard ? signal_on["out"];
                )
            })
            .collect::<Vec<_>>();
        builder
            .component
            .continuous_assignments
            .append(&mut cleanup);

        // Done conditional for this group.
        let done = builder.build_assignment(
            par_group.borrow().get("done"),
            signal_on.borrow().get("out"),
            done_guard,
        );
        par_group.borrow_mut().assignments.push(done);

        // Add NODE_ID to compiled group.
        let mut en = ir::Control::enable(par_group);
        let node_id = s.attributes.get(NODE_ID).unwrap();
        en.get_mut_attributes().insert(NODE_ID, node_id);

        Ok(Action::change(en))
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let control = Rc::clone(&comp.control);
        let attrs = comp.attributes.clone();

        let mut builder = ir::Builder::new(comp, sigs);
        let mut sch = Schedule::from(&mut builder);

        // Add assignments for the final states
        sch.calculate_states(&control.borrow(), self.early_transitions)?;
        let fsm_impl = self.get_representation(&sch, &attrs);
        let comp_group =
            sch.realize_schedule(self.dump_fsm, &mut self.fsm_groups, fsm_impl);

        Ok(Action::change(ir::Control::enable(comp_group)))
    }

    /// If requested, emit FSM json after all components are processed
    fn finish_context(&mut self, _ctx: &mut calyx_ir::Context) -> VisResult {
        if let Some(json_out_file) = &self.dump_fsm_json {
            let _ = serde_json::to_writer_pretty(
                json_out_file.get_write(),
                &self.fsm_groups,
            );
        }
        Ok(Action::Continue)
    }
}
