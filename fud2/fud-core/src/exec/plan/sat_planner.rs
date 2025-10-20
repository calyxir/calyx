//! A SAT solver based planner. This encodes states as variables. If and only if a state is ever
//! created, that state's corresponding variable must be true.
//!
//! The encoding by looking at all of the ops which can create a given state. Call the inputs to
//! one of those ops a dependency for the state. Looking at every ops, every state will have many
//! dependencies, and if a state is chosen, at least one of those dependencies must be fulfilled.
//! This can be modeled by the state implying all of its dependencies.

use std::collections::HashMap;

use cranelift_entity::{PrimaryMap, SecondaryMap};
use rustsat::{
    instances::SatInstance,
    solvers::{Solve, SolverResult},
    types::{Assignment, Lit, TernaryVal},
};

use super::{
    super::{OpRef, Operation, State, StateRef},
    FindPlan, PlannerType,
    planner::Step,
};

struct DepClauses<'a> {
    state_of_lit: HashMap<Lit, StateRef>,
    lit_of_state: HashMap<StateRef, Lit>,
    op_of_lit: HashMap<Lit, OpRef>,
    lit_of_op: HashMap<OpRef, Lit>,

    ops: &'a PrimaryMap<OpRef, Operation>,

    made_from: HashMap<Lit, Vec<Lit>>,
    instance: SatInstance,
}

impl<'a> DepClauses<'a> {
    pub fn from_ops(ops: &'a PrimaryMap<OpRef, Operation>) -> Self {
        DepClauses {
            state_of_lit: HashMap::new(),
            lit_of_state: HashMap::new(),
            op_of_lit: HashMap::new(),
            lit_of_op: HashMap::new(),
            ops,
            made_from: HashMap::new(),
            instance: SatInstance::new(),
        }
    }

    pub fn state_lit(&mut self, s: StateRef) -> Lit {
        if !self.lit_of_state.contains_key(&s) {
            let lit = self.instance.new_lit();
            self.lit_of_state.insert(s, lit);
            self.state_of_lit.insert(lit, s);
            lit
        } else {
            self.lit_of_state[&s]
        }
    }

    pub fn op_lit(&mut self, o: OpRef) -> Lit {
        if !self.lit_of_op.contains_key(&o) {
            let lit = self.instance.new_lit();
            self.lit_of_op.insert(o, lit);
            self.op_of_lit.insert(lit, o);
            lit
        } else {
            self.lit_of_op[&o]
        }
    }

    pub fn add_dep(&mut self, state_id: StateRef, dep_ref: OpRef) {
        let ls = self.state_lit(state_id);
        let dep = &self.ops[dep_ref];
        let ld: Vec<Lit> =
            dep.input.iter().map(|&d| self.state_lit(d)).collect();
        let op_lit = self.op_lit(dep_ref);
        self.instance.add_cube_impl_cube(&[ls, op_lit], &ld);
        self.made_from.entry(ls).or_default().push(op_lit);
    }

    pub fn instance(
        &mut self,
        inputs: &[StateRef],
        outputs: &[StateRef],
        through: &[OpRef],
    ) -> SatInstance {
        let mut out_instance = self.instance.clone();
        // We must take outputs.
        for &output in outputs {
            let lo = self.state_lit(output);
            out_instance.add_unit(lo);
        }
        // We need to mark primitive inputs, ones with no deps as things which can never be taken.
        for (&lit, state_id) in &self.state_of_lit {
            if !inputs.contains(state_id) || outputs.contains(state_id) {
                let ops_making_lit = self.made_from.entry(lit).or_default();
                if !ops_making_lit.is_empty() {
                    out_instance.add_lit_impl_clause(lit, &ops_making_lit[..]);
                } else {
                    let neg = Lit::negative(lit.vidx32());
                    out_instance.add_unit(neg);
                }
            }
        }
        // We need to make sure all the ops in through are taken.
        for &op in through {
            out_instance.add_unit(self.op_lit(op));
        }
        out_instance
    }
}

struct Planner<'a> {
    /// A map from a state to a list of dependencies.
    ops: &'a PrimaryMap<OpRef, Operation>,
    dep_clauses: DepClauses<'a>,
}

impl<'a> Planner<'a> {
    pub fn from_ops(ops: &'a PrimaryMap<OpRef, Operation>) -> Self {
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

        Self { ops, dep_clauses }
    }

    fn solve(
        &mut self,
        inputs: &[StateRef],
        outputs: &[StateRef],
        through: &[OpRef],
    ) -> Option<Assignment> {
        let instance = self.dep_clauses.instance(inputs, outputs, through);
        let mut solver = rustsat_minisat::core::Minisat::default();
        solver.add_cnf(instance.into_cnf().0).unwrap();
        match solver.solve().unwrap() {
            SolverResult::Sat => Some(solver.full_solution().unwrap()),
            SolverResult::Unsat => None,
            SolverResult::Interrupted => None,
        }
    }

    fn assignment_to_plan(mut self, a: Assignment) -> Vec<Step> {
        self.ops
            .iter()
            .filter_map(|(op_ref, op)| {
                let op_taken = matches!(
                    a[self.dep_clauses.op_lit(op_ref).var()],
                    TernaryVal::True
                );
                if op_taken {
                    let mut used_states = op.output.clone();
                    used_states.retain(|s| {
                        matches!(
                            a[self.dep_clauses.state_lit(*s).var()],
                            TernaryVal::True
                        )
                    });
                    Some((op_ref, used_states))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct SatPlanner {}

impl FindPlan for SatPlanner {
    fn find_plan(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
        _states: &PrimaryMap<StateRef, State>,
    ) -> Option<Vec<Step>> {
        let mut planner = Planner::from_ops(ops);
        planner
            .solve(start, end, through)
            .map(|a| planner.assignment_to_plan(a))
            // The SAT encoding does not allow input states to be used as is unless there is no op
            // which can make them. Therefore, if the plan is empty, the only way to create a valid
            // plan is to not apply any ops. This is interpreted as no plan existing so the planner
            // should return `None`.
            .take_if(|plan| !plan.is_empty())
    }

    fn ty(&self) -> PlannerType {
        PlannerType::Sat
    }
}
