use crate::flatten::{
    flat_ir::prelude::CellInfo, structures::environment::Environment,
};

use super::{prim_trait::DummyPrimitive, Primitive};

pub fn build_primitive(
    _prim: &CellInfo,
    _env: &Environment,
) -> Box<dyn Primitive> {
    return DummyPrimitive::new_dyn();

    #[allow(unreachable_code)]
    match &_prim.prototype {
        crate::flatten::flat_ir::cell_prototype::CellPrototype::Constant {
            value: _,
            width: _,
            c_type: _,
        } => todo!(),
        crate::flatten::flat_ir::cell_prototype::CellPrototype::Register {
            width: _,
        } => todo!(),
        crate::flatten::flat_ir::cell_prototype::CellPrototype::Unknown(
            _,
            _,
        ) => todo!(),
        crate::flatten::flat_ir::cell_prototype::CellPrototype::Component(
            _,
        ) => unreachable!(
            "Build primitive erroneously called on a calyx component"
        ),
    }
}
