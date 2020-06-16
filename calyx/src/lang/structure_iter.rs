use super::structure::DataDirection;
use petgraph::graph::NodeIndex;

/// TODO(rachit): Implement filtering with group name.
#[derive(Clone, Debug)]
pub struct ConnectionIteration {
    /// Throw errors if trying to overwrite a field.
    guarded: bool,
    /// Only iterate over edges that have this particular node.
    pub from_node: Option<NodeIndex>,
    /// Only iterate over edges that have this particlary port.
    pub with_port: Option<String>,
    /// Only iterate over edges where data flows from a port in the given
    /// direction.
    pub direction: Option<DataDirection>,
}

impl Default for ConnectionIteration {
    fn default() -> Self {
        ConnectionIteration {
            guarded: true,
            from_node: None,
            with_port: None,
            direction: None,
        }
    }
}

impl ConnectionIteration {
    /// Iterate over set of edges that contain this edge.
    pub fn with_component(mut self, component: NodeIndex) -> Self {
        if self.guarded && self.from_node.is_some() {
            panic!("Tried to overwrite with_component field in EdgeIterationBuilder")
        }
        self.from_node = Some(component);
        self
    }

    pub fn with_port(mut self, port: String) -> Self {
        if self.guarded && self.with_port.is_some() {
            panic!(
                "Tried to overwrite with_port field in EdgeIterationBuilder"
            );
        }
        self.with_port = Some(port);
        self
    }

    pub fn in_direction(mut self, direction: DataDirection) -> Self {
        if self.guarded && self.with_port.is_some() {
            panic!("Tried to overwrite direction field in EdgeIterationBuilder")
        }
        self.direction = Some(direction);
        self
    }

    /// Disable the guard checking for this iteration builder.
    pub fn disable_guard(mut self) -> Self {
        self.guarded = false;
        self
    }
}
