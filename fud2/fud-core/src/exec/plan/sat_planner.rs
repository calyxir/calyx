//! A SAT solver based planner. This encodes states as variables. If and only if a state is ever
//! created, that state's corresponding variable must be true.
//!
//! The encoding by looking at all of the ops which can create a given state. Call the inputs to
//! one of those ops a dependency for the state. Looking at every ops, every state will have many
//! dependencies, and if a state is chosen, at least one of those dependencies must be fulfilled.
//! This can be modeled by the state implying all of its dependencies.
//!
//! To do so, the planner makes the observation that an op which returns multiple states can be
//! broken into a bunch of ops which return one state. The nomenclature I use is to call these
//! dependencies. Precisely, if state `s1` can be made using `op1`, I say all of the inputs to
//! `op1` form a dependency of `s1`. Therefore over all the ops, each state gets a bunch of
//! possible dependencies, different ways to make it, using different ops.
//!
//! Dependencies are straightforward to encode in boolean logic. Let each op and state be
//! variables. If that variable is true, that state is taken or that op is used. Let `s1` be the
//! variable representing a state with possible dependencies `s2, s3` using `op1` and `s1, s4`
//! using `op2`. This can be encoded as follows: `(s1 ^ op1 => s2 ^ s3) => (s1 ^ op2 => s1 ^ s4)`.
//!
//! Putting an op as a condition of the implication might feel counter-intuitive, but it is
//! necessary. Using the encoding `(s1 => s2 ^ s3 ^ op1) v (s1 => s1 ^ s4 ^ op2)` would allow a
//! solution were ops are taken but never used, for example the following variables being true:
//! `s1, op1, op2, s4`.
//!
//! To respond to a request with given input states, output states, and required ops. First, each
//! output state is conjoined together and added to the expression. Then for each possible state a
//! file could be in, if that state is not an input (or an output*) and cannot be constructed with
//! any ops, the state's negation is conjoined to the boolean expression, representing that the
//! state can never be constructed. If the state can be constructed, taking the state implies one
//! of the ops is used. This is then conjoined to the boolean expression. To encode the required
//! ops, they are simply conjoined to the final expression.
//!
//! As an example, say the same ops as above are used. The desired output is `s1` and the input is
//! `s2, s3` and the required op is `op1`. The final encoding would be:
//! ```text
//! (s1 ^ op1 => s2 ^ s3) ^ (s1 ^ op2 => s1 ^ s4) ^ op1 ^ s1 ^ ~s4 ^ (s1 => op1 ^ op2)
//! ```

use std::collections::HashMap;

use cranelift_entity::{PrimaryMap, SecondaryMap};
use rustsat::{
    instances::SatInstance,
    solvers::{Solve, SolverResult},
    types::{Assignment, Lit, TernaryVal},
};

use crate::exec::plan::op_list_converter::prog_from_op_list;

use super::{
    super::{OpRef, Operation, State, StateRef},
    FindPlan,
    planner::Step,
};

#[derive(Default, Clone)]
struct LitMap {
    /// A map from boolean variables to the state they represent.
    state_of_lit: HashMap<Lit, StateRef>,

    /// A map from a state to the boolean variable representing it.
    lit_of_state: HashMap<StateRef, Lit>,

    /// A map from an op to the boolean variable representing it.
    lit_of_op: HashMap<OpRef, Lit>,
}

impl LitMap {
    pub fn state_lit_or_make(
        &mut self,
        s: StateRef,
        instance: &mut SatInstance,
    ) -> Lit {
        if !self.lit_of_state.contains_key(&s) {
            let lit = instance.new_lit();
            self.lit_of_state.insert(s, lit);
            self.state_of_lit.insert(lit, s);
            lit
        } else {
            self.lit_of_state[&s]
        }
    }

    pub fn op_lit_or_make(
        &mut self,
        o: OpRef,
        instance: &mut SatInstance,
    ) -> Lit {
        if !self.lit_of_op.contains_key(&o) {
            let lit = instance.new_lit();
            self.lit_of_op.insert(o, lit);
            lit
        } else {
            self.lit_of_op[&o]
        }
    }

    pub fn state_lit(&self, s: StateRef) -> Lit {
        self.lit_of_state[&s]
    }

    pub fn op_lit(&self, o: OpRef) -> Lit {
        self.lit_of_op[&o]
    }
}

/// Builder for the big boolean expression checked by the planner.
struct DepClauses<'a> {
    lit_map: LitMap,

    /// A map from a variable representing a state to that variables representing that variable's
    /// dependencies.
    made_from: HashMap<Lit, Vec<Lit>>,

    ops: &'a PrimaryMap<OpRef, Operation>,
    instance: SatInstance,
}

impl<'a> DepClauses<'a> {
    pub fn from_ops(ops: &'a PrimaryMap<OpRef, Operation>) -> Self {
        DepClauses {
            lit_map: LitMap::default(),
            ops,
            made_from: HashMap::new(),
            instance: SatInstance::new(),
        }
    }

    /// Returns the variable representing a state, creating one if it doesn't already exist.
    pub fn state_lit(&mut self, s: StateRef) -> Lit {
        self.lit_map.state_lit_or_make(s, &mut self.instance)
    }

    /// Returns the variable representing an op, creating one if it doesn't already exist.
    pub fn op_lit(&mut self, o: OpRef) -> Lit {
        self.lit_map.op_lit_or_make(o, &mut self.instance)
    }

    /// Adds a dependency to the boolean expression for which `state_id` depends on the inputs of
    /// `dep_ref`.
    pub fn add_dep(&mut self, state_id: StateRef, dep_ref: OpRef) {
        let ls = self.state_lit(state_id);
        let dep = &self.ops[dep_ref];
        let ld: Vec<Lit> =
            dep.input.iter().map(|&d| self.state_lit(d)).collect();
        let op_lit = self.op_lit(dep_ref);
        self.instance.add_cube_impl_cube(&[ls, op_lit], &ld);
        self.made_from.entry(ls).or_default().push(op_lit);
    }

    /// Returns a `SatInstance` representing a request using deps previously added to `self` and
    /// `inputs`, `outputs`, and `through`.
    pub fn instance(
        &self,
        inputs: &[StateRef],
        outputs: &[StateRef],
        through: &[OpRef],
    ) -> (SatInstance, LitMap) {
        let mut out_instance = self.instance.clone();
        let mut out_map = self.lit_map.clone();
        let mut made_from = self.made_from.clone();

        // Require outputs to be created.
        for &output in outputs {
            let lo = out_map.state_lit_or_make(output, &mut out_instance);
            out_instance.add_unit(lo);
        }

        // Mark inputs ones with no deps as things which can never be taken. If there are deps,
        // make sure whenever if that state is constructed, some ops is used to actually create it.
        for (&lit, state_id) in &out_map.state_of_lit {
            // The case `outputs.contains(state_id)` must exist because an output must be
            // constructed even if it is already present as an input.
            if !inputs.contains(state_id) || outputs.contains(state_id) {
                let ops_making_lit = made_from.entry(lit).or_default();
                if !ops_making_lit.is_empty() {
                    out_instance.add_lit_impl_clause(lit, &ops_making_lit[..]);
                } else {
                    let neg = Lit::negative(lit.vidx32());
                    out_instance.add_unit(neg);
                }
            }
        }

        // Make sure all the ops in through are taken.
        for &op in through {
            let lit = out_map.op_lit_or_make(op, &mut out_instance);
            out_instance.add_unit(lit);
        }

        (out_instance, out_map)
    }
}

fn assignment_to_plan(
    a: &Assignment,
    ops: &PrimaryMap<OpRef, Operation>,
    lit_map: &LitMap,
) -> Vec<Step> {
    ops.iter()
        .filter_map(|(op_ref, op)| {
            let op_taken =
                matches!(a[lit_map.op_lit(op_ref).var()], TernaryVal::True);
            if op_taken {
                let mut used_states = op.output.clone();
                used_states.retain(|s| {
                    matches!(a[lit_map.state_lit(*s).var()], TernaryVal::True)
                });
                Some((op_ref, used_states))
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug)]
pub struct SatPlanner {}

impl FindPlan for SatPlanner {
    fn find_plan(
        &self,
        req: &super::Request,
        ops: &PrimaryMap<OpRef, Operation>,
        states: &PrimaryMap<StateRef, State>,
    ) -> Option<crate::flang::Plan> {
        let deps = ops.iter().fold(
            SecondaryMap::new(),
            |acc: SecondaryMap<StateRef, Vec<_>>, (op_ref, op)| {
                op.output.iter().fold(acc, |mut acc, &output_state| {
                    acc[output_state].push(op_ref);
                    acc
                })
            },
        );

        let mut dep_clauses = DepClauses::from_ops(ops);
        for (s, deps) in deps.iter() {
            for dep in deps {
                dep_clauses.add_dep(s, *dep);
            }
        }

        let (instance, lit_map) =
            dep_clauses.instance(req.start_states, req.end_states, req.through);
        let mut solver = rustsat_minisat::core::Minisat::default();
        solver.add_cnf(instance.into_cnf().0).unwrap();
        let assignments = match solver.solve().unwrap() {
            SolverResult::Sat => {
                Some((solver.full_solution().unwrap(), lit_map))
            }
            SolverResult::Unsat => None,
            SolverResult::Interrupted => None,
        };

        let op_list = assignments
            .map(|(a, m)| assignment_to_plan(&a, ops, &m))
            // The SAT encoding does not allow input states to be used as is unless there is no op
            // which can make them. Therefore, if the plan is empty, the only way to create a valid
            // plan is to not apply any ops. This is interpreted as no plan existing so the planner
            // should return `None`.
            .take_if(|plan| !plan.is_empty());

        op_list.map(|l| prog_from_op_list(&l, req, ops, states))
    }
}
