use crate::traversal::{Action, ConstructVisitor, Named, Visitor};
pub struct RedirectFSMConstants {}

impl Named for RedirectFSMConstants {
    fn name() -> &'static str {
        "redirect-fsm-constants"
    }
    fn description() -> &'static str {
        "Reroutes references to constants within an FSM to read from a wire"
    }
}
