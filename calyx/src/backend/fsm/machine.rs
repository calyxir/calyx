#[allow(unused)]
pub struct FSM {
    states: Vec<State>,
    start: State,
}

type Bits = Vec<i64>;

type Edge = (Bits, State);

#[allow(unused)]
pub struct State {
    outputs: Vec<i64>,
    transitions: Vec<Edge>,
    default: Box<State>, // Default next state if no edges are matched
}

#[allow(unused)]
impl State {
    fn transition(st: State, b: Bits) -> State {
        for (bits, next_st) in st.transitions {
            if b == bits {
                return next_st;
            }
        }
        *st.default
    }
}
