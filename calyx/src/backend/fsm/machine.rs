use crate::lang::ast::Id;
use std::collections::HashMap;

/// Represents a pointer to a State in an FSM
#[derive(PartialEq, Clone, Copy, Debug, Hash, Eq)]
pub struct StateIndex {
    id: i64,
}

// pub type Port = (Id, String);
pub type ValuedPort = (Id, String, i64);
pub type Edge = (Vec<ValuedPort>, StateIndex);

/// Represents a State in the FSM. `outputs` represent
/// the wires to enable when this state is activated.
/// `transitions` are a list of `Edge`s which describe which
/// state to transition to and under what conditions
/// `default` is the state to transition to if no other transitions
/// can be taken.
#[derive(PartialEq, Debug, Clone)]
pub struct State {
    outputs: Vec<ValuedPort>,
    transitions: Vec<Edge>,
    default: Option<StateIndex>,
}

/// A representation of an FSM that uses a HashMap to store
/// states. You manipulate the states by using `StateIndex`
/// structs that are received from `FSM::new()` and `fsm.new_state()`.
#[derive(Clone, Debug)]
pub struct FSM {
    pub inputs: Vec<Id>,
    pub outputs: Vec<Id>,
    pub states: HashMap<StateIndex, State>,
    pub start: StateIndex,
    last_index: StateIndex,
}

// Impls for structs

impl StateIndex {
    fn new() -> Self {
        StateIndex { id: 0 }
    }

    fn incr(self) -> Self {
        StateIndex { id: self.id + 1 }
    }
}

#[allow(unused)]
impl State {
    pub fn empty() -> Self {
        State {
            outputs: vec![],
            transitions: vec![],
            default: None,
        }
    }

    pub fn push_output(&mut self, output: ValuedPort) {
        self.outputs.push(output)
    }

    pub fn add_outputs(&mut self, outputs: &mut Vec<ValuedPort>) {
        self.outputs.append(outputs);
    }

    pub fn add_transition(&mut self, edge: Edge) {
        self.transitions.push(edge);
    }
}

impl FSM {
    pub fn new() -> (StateIndex, Self) {
        let mut states = HashMap::new();
        let idx = StateIndex::new();
        states.insert(idx, State::empty());
        (
            idx,
            FSM {
                inputs: vec![],
                outputs: vec![],
                states,
                start: idx,
                last_index: idx,
            },
        )
    }

    #[allow(unused)]
    fn transition(&self, st: StateIndex, i: Vec<ValuedPort>) -> StateIndex {
        for (inputs, next_st) in &self.borrow_state(st).transitions {
            if i == *inputs {
                return *next_st;
            }
        }
        match &self.borrow_state(st).default {
            None => st,
            Some(default) => *default,
        }
    }

    pub fn borrow_state(&self, idx: StateIndex) -> &State {
        self.states.get(&idx).unwrap()
    }

    // XXX(sam), soemthing better than unwrap pls
    pub fn get_state(&mut self, idx: StateIndex) -> &mut State {
        self.states.get_mut(&idx).unwrap()
    }

    pub fn new_state(&mut self) -> StateIndex {
        let new_idx = self.last_index.incr();
        self.states.insert(new_idx, State::empty());
        self.last_index = new_idx;
        new_idx
    }
    // Returns a unique value for the state for rtl generation
    // fn state_value(&self, st: State) -> usize {
    //     (*self.states)
    //         .iter()
    //         .position(
    //             |state| *state == st,
    //             //match st.clone().default {
    //             //None => *state == *st,
    //             //Some(default) => *state == *default,
    //             //})
    //         )
    //         .unwrap()
    //         + 1 // Plus one for 1 indexing (instead of 0 indexing)
    // }

    // Returns the number of bits required to represent each state in the FSM
    // pub fn state_bits(&self) -> i64 {
    //     let num_states: f64 = self.states.len() as f64;
    //     num_states.log2().ceil() as i64
    // }

    // Convenience function for generating verilog string values for each state
    // pub fn state_string(&self, st: State) -> String {
    //     format!("{}'d{}", self.state_bits(), self.state_value(st))
    // }
}
