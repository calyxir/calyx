use crate::values::Value;
use calyx::ir;
use itertools::Itertools;
use serde::Serialize;

/// A primitive for the interpreter.
/// Roughly corresponds to the cells defined in the primitives library for the Calyx compiler.
/// Primitives can be either stateful or combinational.
pub trait Primitive {
    /// Does nothing for comb. prims; mutates internal state for stateful
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)>;

    /// Returns true if this primitive is combinational
    fn is_comb(&self) -> bool;

    /// Validate inputs to the component.
    fn validate(&self, inputs: &[(ir::Id, &Value)]);

    /// Execute the component.
    fn execute(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)>;

    /// Execute the component.
    fn validate_and_execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, Value)> {
        self.validate(inputs);
        self.execute(inputs)
    }

    /// Reset the component.
    fn reset(&mut self, inputs: &[(ir::Id, &Value)]) -> Vec<(ir::Id, Value)>;

    /// Serialize the state of this primitive, if any.
    fn serialize(&self) -> Serializeable {
        Serializeable::Empty
    }

    // more efficient to override this with true in stateful cases
    fn has_serializeable_state(&self) -> bool {
        self.serialize().has_state()
    }
}

/// An enum wrapping over a tuple representing the shape of a multi-dimensional
/// array
#[derive(Clone)]
pub enum Shape {
    D1((usize,)),
    D2((usize, usize)),
    D3((usize, usize, usize)),
    D4((usize, usize, usize, usize)),
}

impl Shape {
    fn is_1d(&self) -> bool {
        matches!(self, Shape::D1(_))
    }
}
impl From<usize> for Shape {
    fn from(u: usize) -> Self {
        Shape::D1((u,))
    }
}
impl From<(usize,)> for Shape {
    fn from(u: (usize,)) -> Self {
        Shape::D1(u)
    }
}
impl From<(usize, usize)> for Shape {
    fn from(u: (usize, usize)) -> Self {
        Shape::D2(u)
    }
}

impl From<(usize, usize, usize)> for Shape {
    fn from(u: (usize, usize, usize)) -> Self {
        Shape::D3(u)
    }
}

impl From<(usize, usize, usize, usize)> for Shape {
    fn from(u: (usize, usize, usize, usize)) -> Self {
        Shape::D4(u)
    }
}

#[derive(Clone)]
pub enum Serializeable {
    Empty,
    Val(u64),
    Array(Vec<u64>, Shape),
}

impl Serializeable {
    pub fn has_state(&self) -> bool {
        !matches!(self, Serializeable::Empty)
    }
}

impl Serialize for Serializeable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Serializeable::Empty => serializer.serialize_unit(),
            Serializeable::Val(u) => u.serialize(serializer),
            Serializeable::Array(arr, shape) => {
                let arr: Vec<&u64> = arr.iter().collect();
                if shape.is_1d() {
                    return arr.serialize(serializer);
                }
                // there's probably a better way to write this
                match shape {
                    Shape::D2(shape) => {
                        let mem = arr
                            .iter()
                            .chunks(shape.1)
                            .into_iter()
                            .map(|x| x.into_iter().collect::<Vec<_>>())
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Shape::D3(shape) => {
                        let mem = arr
                            .iter()
                            .chunks(shape.2 * shape.1)
                            .into_iter()
                            .map(|x| {
                                x.into_iter()
                                    .chunks(shape.1)
                                    .into_iter()
                                    .map(|y| y.into_iter().collect::<Vec<_>>())
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Shape::D4(shape) => {
                        let mem = arr
                            .iter()
                            .chunks(shape.3 * shape.2 * shape.1)
                            .into_iter()
                            .map(|x| {
                                x.into_iter()
                                    .chunks(shape.2 * shape.1)
                                    .into_iter()
                                    .map(|y| {
                                        y.into_iter()
                                            .chunks(shape.1)
                                            .into_iter()
                                            .map(|z| {
                                                z.into_iter()
                                                    .collect::<Vec<_>>()
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        mem.serialize(serializer)
                    }
                    Shape::D1(_) => unreachable!(),
                }
            }
        }
    }
}

impl Serialize for dyn Primitive {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.serialize().serialize(serializer)
    }
}
