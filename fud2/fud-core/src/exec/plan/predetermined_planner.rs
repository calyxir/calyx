//! A planner which is predetermined by input from stdin. This always returns `None` for the plan
//! and must be special cased by the logic later. Currently planners only emit states with file
//! assignment done later.

use super::{FindPlan, PlannerType};

#[derive(Debug)]
pub struct PredeterminedPlanner {}

impl FindPlan for PredeterminedPlanner {
    fn find_plan(
        &self,
        _start: &[crate::exec::StateRef],
        _end: &[crate::exec::StateRef],
        _through: &[crate::exec::OpRef],
        _ops: &cranelift_entity::PrimaryMap<
            crate::exec::OpRef,
            crate::exec::Operation,
        >,
        _states: &cranelift_entity::PrimaryMap<
            crate::exec::StateRef,
            crate::exec::State,
        >,
    ) -> Option<Vec<super::Step>> {
        None
    }

    fn ty(&self) -> PlannerType {
        PlannerType::Predetermined
    }
}
