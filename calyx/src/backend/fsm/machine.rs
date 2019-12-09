use crate::lang::ast::Id;

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct FSM<'a> {
    pub inputs: Vec<Id>,
    pub outputs: Vec<Id>,
    pub states: Box<Vec<State<'a>>>,
    pub start: &'a State<'a>,
}

// The String is the name of the input port, and
// must be an input of the toplevel component for
// this FSM. The i64 is the value that it needs
// to be to help trigger state transition

//type Bits = Vec<i64>;

pub type Edge<'a> = (Vec<Input>, &'a State<'a>);
pub type Port = (Id, String);

pub type Input = (Port, i64);

#[allow(unused)]
#[derive(PartialEq, Debug, Clone)]
pub struct State<'a> {
    pub outputs: Vec<(Port, i64)>,
    pub transitions: Vec<Edge<'a>>,
    //pub default: &'a State<'a>,
}

#[allow(unused)]
impl State<'_> {
    pub fn empty() -> Self {
        State {
            outputs: vec![],
            transitions: vec![],
            //default: Box::new(),
        }
    }

    fn transition<'a>(st: &'a State<'a>, i: Vec<Input>) -> &'a State<'a> {
        for (inputs, next_st) in &st.transitions {
            if i == *inputs {
                return next_st.clone();
            }
        }
        panic!("not found matching state")
        // match st.default {
        //     None => &st,
        //     Some(default) => default,
        // }
    }
}

#[allow(unused)]
impl<'a> FSM<'a> {
    pub fn new(start: &'a State<'a>) -> Self {
        FSM {
            inputs: vec![],
            outputs: vec![],
            states: Box::new(vec![]),
            start,
        }
    }
    // Returns a unique value for the state for rtl generation
    fn state_value(&self, st: &State) -> usize {
        (*self.states)
            .iter()
            .position(
                |state| *state == *st, //match st.clone().default {
                                       //None => *state == *st,
                                       //Some(default) => *state == *default,
                                       //})
            )
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
