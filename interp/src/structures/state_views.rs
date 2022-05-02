//! Structures and Traits for viewing the state of the interpreter environment
//!
//! Unless you REALLY need to be here, all this irritating stuff is best left
//! alone because it is fiddly and not all that interesting.

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use crate::{
    debugger::{name_tree::ActiveTreeNode, PrintCode},
    environment::{InterpreterState, PrimitiveMap},
    interpreter::ConstCell,
    interpreter_ir as iir,
    primitives::{Entry, Primitive, Serializeable},
    utils::AsRaw,
    values::Value,
};
use calyx::ir::{self, RRC};
use serde::Serialize;

use super::names::GroupQIN;

/// A concrete type wrapping a single borrowed reference and a vector of states.
/// The former corresponds to the root environment of a par split while the
/// latter contains the views for each par child.
#[derive(Clone)]
pub struct CompositeView<'a>(&'a InterpreterState, Vec<StateView<'a>>);

impl<'a, 'outer> CompositeView<'a> {
    /// Basic constructor for the struct
    pub fn new(state: &'a InterpreterState, vec: Vec<StateView<'a>>) -> Self {
        Self(state, vec)
    }
}

/// An enum type wrapping the two possible concrete immutable state views. This
/// type implements the [State] trait.
#[derive(Clone)]
pub enum StateView<'inner> {
    /// Variant for a single [InterpreterState]
    SingleView(&'inner InterpreterState),
    /// Variant for state views which correspond to multiple par branches
    Composite(CompositeView<'inner>),
}

/// The mutable analogue to [CompositeView]. As with that struct, the first
/// reference is the root environment of the par split and the latter vec is the
/// environment for each of the par children.
pub struct MutCompositeView<'a>(
    &'a mut InterpreterState,
    Vec<MutStateView<'a>>,
);

/// The mutable analogue to [StateView].
pub enum MutStateView<'inner> {
    /// Variant for a single [InterpreterState]
    Single(&'inner mut InterpreterState),
    /// Variant for a composite view corresponding to the state during the
    /// execution of a par block
    Composite(MutCompositeView<'inner>),
}

impl<'inner> MutCompositeView<'inner> {
    /// Basic constructor for the struct
    pub fn new(
        state: &'inner mut InterpreterState,
        vec: Vec<MutStateView<'inner>>,
    ) -> Self {
        Self(state, vec)
    }

    /// Updates the value of the given port to the given value in the
    /// environment state. Note that this means updating the value in all arms
    /// of the children and the root state (this latter point is needed to avoid
    /// issues)
    pub fn insert<P>(&mut self, port: P, value: Value)
    where
        P: AsRaw<ir::Port>,
    {
        let raw = port.as_raw();
        self.0.insert(raw, value.clone());
        for view in self.1.iter_mut() {
            view.insert(raw, value.clone())
        }
    }
}

impl<'a> From<&'a mut InterpreterState> for MutStateView<'a> {
    fn from(env: &'a mut InterpreterState) -> Self {
        Self::Single(env)
    }
}

impl<'a> From<MutCompositeView<'a>> for MutStateView<'a> {
    fn from(mv: MutCompositeView<'a>) -> Self {
        Self::Composite(mv)
    }
}

impl<'a> MutStateView<'a> {
    /// Updates the value of the given port to the given value for this state
    /// view.
    pub fn insert<P: AsRaw<ir::Port>>(&mut self, port: P, value: Value) {
        match self {
            MutStateView::Single(s) => s.insert(port, value),
            MutStateView::Composite(c) => c.insert(port, value),
        }
    }
}

impl<'a, 'outer> From<&'a InterpreterState> for StateView<'a> {
    fn from(env: &'a InterpreterState) -> Self {
        Self::SingleView(env)
    }
}

impl<'a> From<CompositeView<'a>> for StateView<'a> {
    fn from(cv: CompositeView<'a>) -> Self {
        Self::Composite(cv)
    }
}

impl<'a> StateView<'a> {
    pub fn lookup<P: AsRaw<ir::Port>>(&self, target: P) -> &Value {
        match self {
            StateView::SingleView(sv) => sv.get_from_port(target),
            StateView::Composite(cv) => match cv.1.len() {
                0 => cv.0.get_from_port(target),
                1 => cv.1[0].lookup(target),
                _ => {
                    let original = cv.0.get_from_port(target.as_raw());
                    let new =
                        cv.1.iter()
                            .filter_map(|x| {
                                let val = x.lookup(target.as_raw());
                                if val == original {
                                    None
                                } else {
                                    Some(val)
                                }
                            })
                            .collect::<Vec<_>>();
                    match new.len() {
                        0 => original,
                        1 => new[0],
                        _ => panic!("conflicting parallel values"),
                    }
                }
            },
        }
    }

    pub fn sub_component_currently_executing(&self) -> HashSet<GroupQIN> {
        match self {
            StateView::SingleView(sv) => sv.sub_component_currently_executing(),
            StateView::Composite(c) => c.0.sub_component_currently_executing(),
        }
    }

    pub fn get_ctx(&self) -> &iir::ComponentCtx {
        match self {
            StateView::SingleView(sv) => &sv.context,
            StateView::Composite(cv) => &cv.0.context,
        }
    }

    pub fn get_cell_map(&self) -> &PrimitiveMap {
        match self {
            StateView::SingleView(sv) => &sv.cell_map,
            StateView::Composite(cv) => &cv.0.cell_map,
        }
    }

    pub fn get_comp(&self) -> &Rc<iir::Component> {
        match self {
            StateView::SingleView(c) => &c.component,
            StateView::Composite(c) => &c.0.component,
        }
    }
    pub fn get_active_tree(&self) -> Vec<ActiveTreeNode> {
        match self {
            StateView::SingleView(c) => c.get_active_tree(),
            StateView::Composite(c) => c.0.get_active_tree(),
        }
    }

    pub fn get_cell_state<R: AsRaw<ir::Cell>>(
        &self,
        cell: R,
        print_code: &PrintCode,
    ) -> Serializeable {
        let map = self.get_cell_map();
        let map_ref = map.borrow();
        map_ref
            .get(&cell.as_raw())
            .map(|x| Primitive::serialize(&**x, Some(*print_code)))
            .unwrap_or(Serializeable::Empty)
    }

    /// Returns a string representing the current state of the environment. This
    /// just serializes the environment to a string and returns that string
    pub fn state_as_str(&self) -> String {
        serde_json::to_string_pretty(&self.gen_serialzer(false)).unwrap()
    }

    pub fn get_cells<S: AsRef<str> + Clone>(
        &self,
        name: &S,
    ) -> Vec<RRC<ir::Cell>> {
        let ctx_ref = self.get_ctx();
        ctx_ref.iter().filter_map(|x| x.find_cell(name)).collect()
    }

    pub fn get_cell<S: AsRef<str> + Clone>(
        &self,
        name: S,
    ) -> Option<RRC<ir::Cell>> {
        match self {
            StateView::SingleView(sv) => sv.component.find_cell(&name),
            StateView::Composite(cv) => cv.0.component.find_cell(&name),
        }
    }

    pub fn gen_serialzer(&self, raw: bool) -> FullySerialize {
        let ctx = self.get_ctx();
        let cell_prim_map = &self.get_cell_map().borrow();

        let bmap: BTreeMap<_, _> = ctx
            .iter()
            .filter(|x| x.name == self.get_comp().name) // there should only be one such comp
            .map(|comp| {
                let inner_map: BTreeMap<_, _> = comp
                    .cells
                    .iter()
                    .map(|cell| {
                        let inner_map: BTreeMap<_, _> = cell
                            .borrow()
                            .ports
                            .iter()
                            .map(|port| {
                                let value = self.lookup(port.as_raw());

                                (
                                    port.borrow().name.clone(),
                                    if port
                                        .borrow()
                                        .attributes
                                        .has("interp_signed")
                                    {
                                        value.as_i64().into()
                                    } else {
                                        value.as_u64().into()
                                    },
                                )
                            })
                            .collect();
                        (cell.borrow().name().clone(), inner_map)
                    })
                    .collect();
                (comp.name.clone(), inner_map)
            })
            .collect();
        let cell_map: BTreeMap<_, _> = ctx
            .iter()
            .filter(|x| x.name == self.get_comp().name)
            .map(|comp| {
                let inner_map: BTreeMap<_, _> = comp
                    .cells
                    .iter()
                    .filter_map(|cell_ref| {
                        let cell = cell_ref.borrow();
                        if cell.get_attribute("external").is_some() {
                            if let Some(prim) = cell_prim_map
                                .get(&(&cell as &ir::Cell as ConstCell))
                            {
                                if !prim.is_comb() {
                                    return Some((
                                        cell.name().clone(),
                                        Primitive::serialize(
                                            &**prim,
                                            raw.then(|| PrintCode::Binary),
                                        ), //TODO Griffin: Fix this
                                    ));
                                }
                            }
                        }
                        None
                    })
                    .collect();
                (comp.name.clone(), inner_map)
            })
            .collect();

        FullySerialize {
            ports: bmap,
            memories: cell_map,
        }
    }
}

#[allow(clippy::borrowed_box)]
#[derive(Serialize, Clone)]
/// Struct to fully serialize the internal state of the environment
pub struct FullySerialize {
    ports: BTreeMap<ir::Id, BTreeMap<ir::Id, BTreeMap<ir::Id, Entry>>>,
    memories: BTreeMap<ir::Id, BTreeMap<ir::Id, Serializeable>>,
}

impl<'a> Serialize for StateView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.gen_serialzer(false).serialize(serializer)
    }
}

impl Serialize for InterpreterState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let sv: StateView = self.into();
        sv.gen_serialzer(false).serialize(serializer)
    }
}
