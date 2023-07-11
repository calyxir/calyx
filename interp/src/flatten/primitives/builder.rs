use crate::{
    flatten::{
        flat_ir::{
            cell_prototype::CellPrototype,
            prelude::{CellInfo, GlobalPortId},
        },
        structures::environment::Environment,
    },
    values::Value,
};

use super::{combinational::*, Primitive};

pub fn build_primitive(
    env: &mut Environment,
    prim: &CellInfo,
    base_port: GlobalPortId,
) -> Box<dyn Primitive> {
    match &prim.prototype {
        CellPrototype::Constant {
            value: val,
            width,
            c_type: _,
        } => {
            let v = Value::from(*val, *width);
            env.ports[base_port] = v.clone();
            Box::new(StdConst::new(v, base_port))
        }

        CellPrototype::Component(_) => unreachable!(
            "Build primitive erroneously called on a calyx component"
        ),
        CellPrototype::SingleWidth { op: _, width: _ } => todo!(),
        CellPrototype::FixedPoint {
            op: _,
            width: _,
            int_width: _,
            frac_width: _,
        } => todo!(),
        CellPrototype::Slice {
            in_width: _,
            out_width: _,
        } => todo!(),
        CellPrototype::Pad {
            in_width: _,
            out_width: _,
        } => todo!(),
        CellPrototype::Cat {
            left: _,
            right: _,
            out: _,
        } => todo!(),
        CellPrototype::MemD1 {
            mem_type: _,
            width: _,
            size: _,
            idx_size: _,
        } => todo!(),
        CellPrototype::MemD2 {
            mem_type: _,
            width: _,
            d0_size: _,
            d1_size: _,
            d0_idx_size: _,
            d1_idx_size: _,
        } => todo!(),
        CellPrototype::MemD3 {
            mem_type: _,
            width: _,
            d0_size: _,
            d1_size: _,
            d2_size: _,
            d0_idx_size: _,
            d1_idx_size: _,
            d2_idx_size: _,
        } => todo!(),
        CellPrototype::MemD4 {
            mem_type: _,
            width: _,
            d0_size: _,
            d1_size: _,
            d2_size: _,
            d3_size: _,
            d0_idx_size: _,
            d1_idx_size: _,
            d2_idx_size: _,
            d3_idx_size: _,
        } => todo!(),
        CellPrototype::Unknown(_, _) => todo!(),
    }
}
