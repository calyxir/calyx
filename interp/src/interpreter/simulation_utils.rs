use crate::utils::{get_const_from_rrc, OutputValueRef};
use crate::values::{OutputValue, ReadableValue, Value};
use calyx::ir;
use calyx::ir::RRC;

pub type ConstPort = *const ir::Port;
pub type ConstCell = *const ir::Cell;

pub fn get_done_port(group: &ir::Group) -> RRC<ir::Port> {
    group.get(&"done")
}

pub fn is_signal_high(done: OutputValueRef) -> bool {
    match done {
        OutputValueRef::ImmediateValue(v) => v.as_u64() == 1,
        OutputValueRef::LockedValue(_) => false,
        OutputValueRef::PulseValue(v) => v.get_val().as_u64() == 1,
    }
}

pub fn get_dst_cells<'a, I>(iter: I) -> Vec<RRC<ir::Cell>>
where
    I: Iterator<Item = &'a ir::Assignment>,
{
    iter.filter_map(|assign| {
        match &assign.dst.borrow().parent {
            ir::PortParent::Cell(c) => {
                match &c.upgrade().borrow().prototype {
                    ir::CellType::Primitive { .. }
                    | ir::CellType::Constant { .. } => Some(c.upgrade()),
                    ir::CellType::Component { .. } => {
                        // TODO (griffin): We'll need to handle this case at some point
                        todo!()
                    }
                    ir::CellType::ThisComponent => None,
                }
            }
            ir::PortParent::Group(_) => None,
        }
    })
    .collect()
}
