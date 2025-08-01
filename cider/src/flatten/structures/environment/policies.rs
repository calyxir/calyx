use argh::FromArgValue;
use itertools::Itertools;
use rand::seq::{IndexedMutRandom, IndexedRandom};

use crate::{
    errors::{RuntimeError, RuntimeResult},
    flatten::structures::environment::program_counter::{
        ProgramCounter, ProgramPointer,
    },
};

pub trait ClonePolicy {
    fn box_clone(&self) -> Box<dyn EvaluationPolicy>;
}

pub(crate) trait EvaluationPolicy: ClonePolicy {
    /// Given the current program counter and a set of prospective new nodes,
    /// decide which of the new nodes should be marked as active or paused nodes
    /// should be given initialized as active
    fn decide_new_nodes(
        &mut self,
        current: &ProgramCounter,
        new: &mut [ProgramPointer],
    ) -> RuntimeResult<()>;

    /// Given the program counter, decide what paused nodes to unpause
    fn decide_unpause(
        &mut self,
        current: &mut ProgramCounter,
    ) -> RuntimeResult<()>;
}

impl<T> ClonePolicy for T
where
    T: EvaluationPolicy + Clone + 'static,
{
    fn box_clone(&self) -> Box<dyn EvaluationPolicy> {
        Box::new(self.clone())
    }
}

/// The standard execution policy which leaves all new nodes enabled and errors
/// if execution halts
#[derive(Debug, Clone, Default)]
pub struct DefaultPolicy;

impl EvaluationPolicy for DefaultPolicy {
    fn decide_new_nodes(
        &mut self,
        _current: &ProgramCounter,
        _new: &mut [ProgramPointer],
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn decide_unpause(
        &mut self,
        _current: &mut ProgramCounter,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::StalledExecution.into())
    }
}

/// For all new nodes randomly decide whether or not to pause them and when
/// stalled, randomly unpause some of the paused nodes
#[derive(Debug, Clone, Default)]
pub struct RandomPolicy;

impl EvaluationPolicy for RandomPolicy {
    fn decide_new_nodes(
        &mut self,
        _current: &ProgramCounter,
        new: &mut [ProgramPointer],
    ) -> RuntimeResult<()> {
        for node in new {
            if rand::random_bool(0.5) {
                node.pause();
            }
        }
        Ok(())
    }

    fn decide_unpause(
        &mut self,
        current: &mut ProgramCounter,
    ) -> RuntimeResult<()> {
        for node in current.vec_mut() {
            if !node.is_enabled() && rand::random_bool(0.5) {
                node.unpause();
            }
        }
        Ok(())
    }
}

/// When adding new nodes, randomly chooses exactly one node to be active. When
/// unpausing, will unpause exactly one paused node
#[derive(Debug, Clone, Default)]
pub struct RandomSerialized;

impl EvaluationPolicy for RandomSerialized {
    fn decide_new_nodes(
        &mut self,
        _current: &ProgramCounter,
        new: &mut [ProgramPointer],
    ) -> RuntimeResult<()> {
        for node in new.iter_mut() {
            node.pause();
        }

        if let Some(node) = new.choose_mut(&mut rand::rng()) {
            node.unpause();
        }

        Ok(())
    }

    fn decide_unpause(
        &mut self,
        current: &mut ProgramCounter,
    ) -> RuntimeResult<()> {
        let paused = current
            .iter()
            .enumerate()
            .filter_map(|(i, x)| (!x.is_enabled()).then_some(i))
            .collect_vec();

        if let Some(idx) = paused.choose(&mut rand::rng()) {
            current.vec_mut()[*idx].unpause();
        } else {
            todo!("Error")
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PolicyChoice {
    Default,
    Random,
    RandomSerialized,
}

impl PolicyChoice {
    pub(crate) fn generate_policy(&self) -> Box<dyn EvaluationPolicy> {
        match self {
            PolicyChoice::Default => Box::new(DefaultPolicy),
            PolicyChoice::Random => Box::new(RandomPolicy),
            PolicyChoice::RandomSerialized => Box::new(RandomSerialized),
        }
    }
}

impl FromArgValue for PolicyChoice {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "normal" | "default" => Ok(PolicyChoice::Default),
            "rand" | "random" => Ok(PolicyChoice::Random),
            "rand_ser" | "serialized" | "random_serialized"
            | "randomserialized" | "serial" | "serial_rand"
            | "serial_random" | "sequential" => {
                Ok(PolicyChoice::RandomSerialized)
            }
            _ => Err(format!("Unknown policy : '{value}'")),
        }
    }
}
