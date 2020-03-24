/// Recursively stores a component's state
#[derive(Debug, Clone)]
pub struct State {
    /// Mapping from instance Ids to states for user-defined subcomponents
    comp_states: HashMap<ast::Id, State>,
    /// Mapping from instance Ids to states for primitive subcomponents
    prim_states: HashMap<ast::Id, Option<i64>>,
}
