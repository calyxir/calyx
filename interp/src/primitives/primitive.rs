use crate::values::{OutputValue, Value};
use calyx::ir;
use ndarray::Array;
use serde::Serialize;
pub trait Primitive {
    /// Returns true if this primitive is combinational
    fn is_comb(&self) -> bool;

    /// Validate inputs to the component.
    fn validate(&self, inputs: &[(ir::Id, &Value)]);

    /// Execute the component.
    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
        done_val: Option<&Value>,
    ) -> Vec<(ir::Id, OutputValue)>;

    /// Reset the component.
    fn reset(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, OutputValue)>;

    /// Transfers the update held in a primitive's buffer into the
    /// state contained within the primitive itself. Until this method is
    /// invoked, the primitive's internal state will remain unchanged by
    /// execution. This is to prevent ephemeral changes due to repeated
    /// invocations
    fn commit_updates(&mut self);

    /// Resets the primitive's update buffer without commiting the held changes,
    /// effectively reverting the write and ensuring it does not occur. Use to
    /// reset stateful primitives after a group execution.
    fn clear_update_buffer(&mut self);

    // stateful things should override this
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
                        let array = Array::from_shape_vec(*shape, arr).unwrap();
                        array.serialize(serializer)
                    }
                    Shape::D3(shape) => {
                        let array = Array::from_shape_vec(*shape, arr).unwrap();
                        array.serialize(serializer)
                    }
                    Shape::D4(shape) => {
                        let array = Array::from_shape_vec(*shape, arr).unwrap();
                        array.serialize(serializer)
                    }
                    _ => unreachable!(),
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
