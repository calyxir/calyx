use crate::lang::ast::Id;
use std::collections::HashMap;

/// Represents a pointer to a State in an FSM
#[derive(PartialEq, Clone, Copy, Debug, Hash, Eq)]
pub struct StateIndex {
    pub id: i64,
}

/// A ValuedPort is (component id, port name, value)
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
    pub outputs: Vec<ValuedPort>,
    pub transitions: Vec<Edge>,
    default: Option<StateIndex>,
}

/// A representation of an FSM that uses a HashMap to store
/// states. You manipulate the states by using `StateIndex`
/// structs that are received from `FSM::new()` and `fsm.new_state()`.
#[derive(Clone, Debug)]
pub struct FSM {
    pub name: String,
    pub states: HashMap<StateIndex, State>,
    start: StateIndex,
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
    pub fn new(name: &str) -> (StateIndex, Self) {
        let mut states = HashMap::new();
        let idx = StateIndex::new();
        states.insert(idx, State::empty());
        (
            idx,
            FSM {
                name: name.to_string(),
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
    // Returns the number of bits required to represent each state in the FSM
    pub fn state_bits(&self) -> i64 {
        let num_states: f64 = self.states.len() as f64;
        num_states.log2().ceil() as i64
    }

    // Convenience function for generating verilog string values for each state
    pub fn state_string(&self, st_ind: StateIndex) -> String {
        format!("{}'d{}", self.state_bits(), st_ind.id)
    }

    /// A vector of all the inputs to the FSM (based off state transition
    /// edges' port fields- ignores component id's)
    pub fn inputs(&self) -> Vec<Id> {
        let mut v: Vec<Id> = Vec::new();
        for (_, st) in &self.states {
            for (ports, _) in &st.transitions {
                for (_, port, _) in ports {
                    if !v.contains(port) {
                        v.push(port.clone())
                    }
                }
            }
        }
        v
    }

    /// A vector of all the outputs to the FSM (based off state
    /// outputs' port fields- ignores component id's)
    pub fn outputs(&self) -> Vec<Id> {
        let mut v: Vec<Id> = Vec::new();
        for (_, st) in &self.states {
            for (_, port, _) in &st.outputs {
                if !v.contains(port) {
                    v.push(port.clone());
                }
            }
        }
        v
    }
}
