use super::{
    ast,
    ast::{Atom, Port},
    context::Context,
    structure::{NodeData, StructureGraph},
};
use crate::errors;
use petgraph::graph::{EdgeIndex, NodeIndex};

/// Implements convience functions to build commonly used ast nodes and
/// add them to the structure graph.
pub trait ASTBuilder {
    /// Abstract representation for the  indexing types used by the underlying
    /// graph representation.
    type ComponentHandle;
    type ConnectionHandle;

    /// Representation of values representing the port.
    type PortRep;

    /// Construct a new primitive of type `prim` with paramters `params`.
    /// The identifier for this component uses the prefix `name_prefix`.
    /// Uses the `ctx` to check the well-formedness of the primitive
    /// instantiation.
    ///
    /// Returns a handle to the component that can be used by the underlying
    /// graph representation to access this new components internal
    /// representation.
    fn new_primitive<S: AsRef<str>>(
        &mut self,
        ctx: &Context,
        name_prefix: S,
        prim: S,
        params: &[u64],
    ) -> errors::Result<Self::ComponentHandle>;

    /// Create a new constant with value `val` and width `width` and add
    /// it to the structure graph. All numbers are represented using
    /// NumType::Decimal.
    ///
    /// Returns a handle to the component for the constant and the default
    /// port on which the constant component outputs values.
    fn new_constant(
        &mut self,
        val: u64,
        width: u64,
    ) -> errors::Result<(Self::ComponentHandle, Self::PortRep)>;

    /// Given a `component` and a `port_name`, return the PortRep for the
    /// port on the component if it exsits.
    fn port_ref<S: AsRef<str>>(
        &self,
        component: Self::ComponentHandle,
        port_name: S,
    ) -> errors::Result<&Self::PortRep>;

    /// Transform a (ComponentHandle, PortRep) pair into an ast::Guard to be
    /// used for guard conditions.
    fn to_guard(
        &self,
        component_port_pair: (Self::ComponentHandle, Self::PortRep),
    ) -> ast::GuardExpr {
        ast::GuardExpr::Atom(self.to_atom(component_port_pair))
    }

    /// Transform a (ComponentHandle, PortRep) pair into an ast::Atom to be
    /// used for guard conditions.
    fn to_atom(
        &self,
        component_port_pair: (Self::ComponentHandle, Self::PortRep),
    ) -> ast::Atom;
}

impl ASTBuilder for StructureGraph {
    type ComponentHandle = NodeIndex;
    type ConnectionHandle = EdgeIndex;
    type PortRep = ast::Id;

    fn new_primitive<S: AsRef<str>>(
        &mut self,
        ctx: &Context,
        name_prefix: S,
        prim: S,
        params: &[u64],
    ) -> errors::Result<NodeIndex> {
        let prim_name = self.namegen.gen_name(name_prefix.as_ref());
        let prim_comp = ctx.instantiate_primitive(
            prim_name.clone(),
            &prim.as_ref().into(),
            params,
        )?;
        Ok(self.add_primitive(
            prim_name.into(),
            prim.as_ref(),
            &prim_comp,
            params,
        ))
    }

    fn new_constant(
        &mut self,
        val: u64,
        width: u64,
    ) -> errors::Result<(NodeIndex, ast::Id)> {
        self.new_constant(val, width)
    }

    fn port_ref<S: AsRef<str>>(
        &self,
        component: NodeIndex,
        port: S,
    ) -> errors::Result<&ast::Id> {
        let node = self.get_node(component);
        node.find_port(&port)
            .ok_or_else(|| errors::Error::UndefinedPort(port.as_ref().into()))
    }

    fn to_atom(&self, (component, port): (NodeIndex, ast::Id)) -> ast::Atom {
        let node = self.get_node(component);
        match &node.data {
            NodeData::Cell(_) => Atom::Port(Port::Comp {
                component: node.name.clone(),
                port,
            }),
            NodeData::Constant(n) => Atom::Num(n.clone()),
            NodeData::Hole(group) => Atom::Port(Port::Hole {
                group: group.clone(),
                name: port,
            }),
            NodeData::Port => Atom::Port(Port::This { port }),
        }
    }
}
