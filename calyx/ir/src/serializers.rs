#[cfg(feature = "serialize")]
use crate::{Cell, Context, Control, IdList, PortParent, RRC};
#[cfg(feature = "serialize")]
use calyx_utils::GetName;
#[cfg(feature = "serialize")]
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};
#[cfg(feature = "serialize")]
use serde_with::SerializeAs;

#[cfg(feature = "serialize")]
impl Serialize for Context {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ctx = ser.serialize_struct("Context", 2)?;
        ctx.serialize_field("components", &self.components)?;
        ctx.serialize_field("entrypoint", &self.entrypoint)?;
        ctx.end()
    }
}

#[cfg(feature = "serialize")]
pub struct SerCellRef;
#[cfg(feature = "serialize")]
impl SerializeAs<RRC<Cell>> for SerCellRef {
    fn serialize_as<S>(
        value: &RRC<Cell>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.borrow().name().serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl SerializeAs<RRC<Control>> for Control {
    fn serialize_as<S>(
        value: &RRC<Control>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.borrow().serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<T: GetName + Serialize> Serialize for IdList<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for obj in self.iter() {
            seq.serialize_element(obj)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serialize")]
impl Serialize for PortParent {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            PortParent::Cell(weak_cell_ref) => {
                weak_cell_ref.upgrade().borrow().name().serialize(ser)
            }
            PortParent::Group(weak_group_ref) => {
                weak_group_ref.upgrade().borrow().name().serialize(ser)
            }
            PortParent::StaticGroup(weak_sgroup_ref) => {
                weak_sgroup_ref.upgrade().borrow().name().serialize(ser)
            }
        }
    }
}
