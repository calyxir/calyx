use super::{
    combinational::*, prim_trait::DummyPrimitive, stateful::*, Primitive,
};
use crate::{
    flatten::{
        flat_ir::{
            cell_prototype::{CellPrototype, MemType, PrimType1},
            prelude::{CellInfo, GlobalPortIdx},
        },
        structures::context::Context,
    },
    serialization::data_dump::DataDump,
    values::Value,
};

pub fn build_primitive(
    prim: &CellInfo,
    base_port: GlobalPortIdx,
    // extras for memory initialization
    ctx: &Context,
    dump: &Option<DataDump>,
) -> Box<dyn Primitive> {
    match &prim.prototype {
        CellPrototype::Constant {
            value: val,
            width,
            c_type: _,
        } => {
            let v = Value::from(*val, *width);
            Box::new(StdConst::new(v, base_port))
        }

        CellPrototype::Component(_) => unreachable!(
            "Build primitive erroneously called on a calyx component"
        ),
        CellPrototype::SingleWidth { op, width } => match op {
            PrimType1::Reg => Box::new(StdReg::new(base_port, *width)),
            PrimType1::Not => Box::new(StdNot::new(base_port)),
            PrimType1::And => Box::new(StdAnd::new(base_port)),
            PrimType1::Or => Box::new(StdOr::new(base_port)),
            PrimType1::Xor => Box::new(StdXor::new(base_port)),
            PrimType1::Add => Box::new(StdAdd::new(base_port)),
            PrimType1::Sub => Box::new(StdSub::new(base_port)),
            PrimType1::Gt => Box::new(StdGt::new(base_port)),
            PrimType1::Lt => Box::new(StdLt::new(base_port)),
            PrimType1::Eq => Box::new(StdEq::new(base_port)),
            PrimType1::Neq => Box::new(StdNeq::new(base_port)),
            PrimType1::Ge => Box::new(StdGe::new(base_port)),
            PrimType1::Le => Box::new(StdLe::new(base_port)),
            PrimType1::Lsh => Box::new(StdLsh::new(base_port, *width)),
            PrimType1::Rsh => Box::new(StdRsh::new(base_port, *width)),
            PrimType1::Mux => Box::new(StdMux::new(base_port)),
            PrimType1::Wire => Box::new(StdWire::new(base_port)),
            PrimType1::SignedAdd => Box::new(StdAdd::new(base_port)),
            PrimType1::SignedSub => Box::new(StdSub::new(base_port)),
            PrimType1::SignedGt => Box::new(StdSgt::new(base_port)),
            PrimType1::SignedLt => Box::new(StdSlt::new(base_port)),
            PrimType1::SignedEq => Box::new(StdSeq::new(base_port)),
            PrimType1::SignedNeq => Box::new(StdSneq::new(base_port)),
            PrimType1::SignedGe => Box::new(StdSge::new(base_port)),
            PrimType1::SignedLe => Box::new(StdSle::new(base_port)),
            PrimType1::SignedLsh => Box::new(StdSlsh::new(base_port)),
            PrimType1::SignedRsh => Box::new(StdSrsh::new(base_port)),
            PrimType1::MultPipe => Box::new(DummyPrimitive),
            PrimType1::SignedMultPipe => todo!(),
            PrimType1::DivPipe => todo!(),
            PrimType1::SignedDivPipe => todo!(),
            PrimType1::Sqrt => todo!(),
            PrimType1::UnsynMult => {
                Box::new(StdUnsynMult::new(base_port, *width))
            }
            PrimType1::UnsynDiv => {
                Box::new(StdUnsynDiv::new(base_port, *width))
            }
            PrimType1::UnsynMod => {
                Box::new(StdUnsynMod::new(base_port, *width))
            }
            PrimType1::UnsynSMult => {
                Box::new(StdUnsynSmult::new(base_port, *width))
            }
            PrimType1::UnsynSDiv => {
                Box::new(StdUnsynSdiv::new(base_port, *width))
            }
            PrimType1::UnsynSMod => {
                Box::new(StdUnsynSmod::new(base_port, *width))
            }
        },
        CellPrototype::FixedPoint {
            op: _,
            width: _,
            int_width: _,
            frac_width: _,
        } => todo!(),
        CellPrototype::Slice {
            in_width: _, // Not actually needed, should probably remove
            out_width,
        } => Box::new(StdSlice::new(base_port, *out_width)),
        CellPrototype::Pad {
            in_width: _, // Not actually needed, should probably remove
            out_width,
        } => Box::new(StdPad::new(base_port, *out_width)),
        CellPrototype::Cat {
            // Turns out under the assumption that the primitive is well formed,
            // none of these parameter values are actually needed
            left: _,
            right: _,
            out: _,
        } => Box::new(StdCat::new(base_port)),
        CellPrototype::Memory {
            mem_type,
            width,
            dims,
        } => {
            let data = dump.as_ref().and_then(|data| {
                let string = ctx.lookup_string(prim.name);
                data.get_data(string)
            });

            match mem_type {
                MemType::Seq => Box::new(if let Some(data) = data {
                    SeqMem::new_with_init(base_port, *width, false, dims, data)
                } else {
                    SeqMemD1::new(base_port, *width, false, dims)
                }),
                MemType::Std => Box::new(if let Some(data) = data {
                    CombMem::new_with_init(base_port, *width, false, dims, data)
                } else {
                    CombMem::new(base_port, *width, false, dims)
                }),
            }
        }

        CellPrototype::Unknown(_, _) => {
            todo!()
        }
    }
}
