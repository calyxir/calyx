use crate::lang::ast::Id;

#[allow(unused)]
pub struct FSM {
    pub inputs: Vec<Id>,
    pub outputs: Vec<Id>,
    pub states: Vec<State>,
    pub start: State,
}

// The String is the name of the input port, and
// must be an input of the toplevel component for
// this FSM. The i64 is the value that it needs
// to be to help trigger state transition
pub type Input = (Id, i64);

//type Bits = Vec<i64>;

pub type Edge = (Vec<Input>, State);

#[allow(unused)]
#[derive(PartialEq, Debug)]
pub struct State {
    pub outputs: Vec<(Id, i64)>,
    pub transitions: Vec<Edge>,
    pub default: Box<State>, // Default next state if no edges are matched
}

#[allow(unused)]
impl State {
    fn transition(st: State, i: Vec<Input>) -> State {
        for (inputs, next_st) in st.transitions {
            if i == inputs {
                return next_st;
            }
        }
        *st.default
    }
}

#[allow(unused)]
impl FSM {
    // Returns a unique value for the state for rtl generation
    fn state_value(&self, st: &State) -> usize {
        self.states
            .iter()
            .position(|state| *state == *st.default)
            .unwrap()
            + 1 // Plus one for 1 indexing (instead of 0 indexing)
    }

    // Returns the number of bits required to represent each state in the FSM
    pub fn state_bits(&self) -> i64 {
        let num_states: f64 = self.states.len() as f64;
        num_states.log2().ceil() as i64
    }

    // Convenience function for generating verilog string values for each state
    pub fn state_string(&self, st: &State) -> String {
        format!("{}'d{}", self.state_bits(), self.state_value(st))
    }
}
