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

use super::{
    combinational::*, prim_trait::DummyPrimitive, stateful::*, Primitive,
};

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
        CellPrototype::SingleWidth { op, width } => todo!(),
        CellPrototype::FixedPoint {
            op,
            width,
            int_width,
            frac_width,
        } => todo!(),
        CellPrototype::Slice {
            in_width,
            out_width,
        } => todo!(),
        CellPrototype::Pad {
            in_width,
            out_width,
        } => todo!(),
        CellPrototype::Cat { left, right, out } => todo!(),
        CellPrototype::MemD1 {
            mem_type,
            width,
            size,
            idx_size,
        } => todo!(),
        CellPrototype::MemD2 {
            mem_type,
            width,
            d0_size,
            d1_size,
            d0_idx_size,
            d1_idx_size,
        } => todo!(),
        CellPrototype::MemD3 {
            mem_type,
            width,
            d0_size,
            d1_size,
            d2_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
        } => todo!(),
        CellPrototype::MemD4 {
            mem_type,
            width,
            d0_size,
            d1_size,
            d2_size,
            d3_size,
            d0_idx_size,
            d1_idx_size,
            d2_idx_size,
            d3_idx_size,
        } => todo!(),
        CellPrototype::Unknown(_, _) => todo!(),
    }
}
