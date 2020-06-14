use super::structure::{DataDirection};
use crate::errors;
use petgraph::graph::{NodeIndex};

pub struct EdgeIterationBuilder {
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

impl Default for EdgeIterationBuilder {
    fn default() -> Self {
        EdgeIterationBuilder {
            guarded: true,
            from_node: None,
            with_port: None,
            direction: None,
        }
    }
}

impl EdgeIterationBuilder {
    /// Iterate over set of edges that contain this edge.
    pub fn with_component(
        &mut self,
        component: NodeIndex,
    ) -> Result<&mut Self, errors::Error> {
        if self.guarded && self.from_node.is_some() {
            return Err(errors::Error::Impossible("Tried to overwrite with_component field in EdgeIterationBuilder".to_string()));
        }
        self.from_node = Some(component);
        Ok(self)
    }

    pub fn with_port(&mut self, port: String) -> Result<&mut Self, errors::Error> {
        if self.guarded && self.with_port.is_some() {
            return Err(errors::Error::Impossible(
                "Tried to overwrite with_port field in EdgeIterationBuilder"
                    .to_string(),
            ));
        }
        self.with_port = Some(port);
        Ok(self)
    }

    pub fn in_direction(
        &mut self,
        direction: DataDirection,
    ) -> Result<&mut Self, errors::Error> {
        if self.guarded && self.with_port.is_some() {
            return Err(errors::Error::Impossible(
                "Tried to overwrite direction field in EdgeIterationBuilder"
                    .to_string(),
            ));
        }
        self.direction = Some(direction);
        Ok(self)
    }

    /// Disable the guard checking for this iteration builder.
    pub fn disable_guard(&mut self) -> &mut Self {
        self.guarded = false;
        self
    }
}
