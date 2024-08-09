use ahash::HashSet;

use super::{combinational::*, stateful::*, Primitive};
use crate::{
    flatten::{
        flat_ir::{
            cell_prototype::{CellPrototype, FXType, MemType, SingleWidthType},
            prelude::{CellInfo, GlobalPortIdx},
        },
        structures::context::Context,
    },
    serialization::DataDump,
    values::Value,
};

pub fn build_primitive(
    prim: &CellInfo,
    base_port: GlobalPortIdx,
    // extras for memory initialization
    ctx: &Context,
    dump: &Option<DataDump>,
    memories_initialized: &mut HashSet<String>,
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
            SingleWidthType::Reg => Box::new(StdReg::new(base_port, *width)),
            SingleWidthType::Not => Box::new(StdNot::new(base_port)),
            SingleWidthType::And => Box::new(StdAnd::new(base_port)),
            SingleWidthType::Or => Box::new(StdOr::new(base_port)),
            SingleWidthType::Xor => Box::new(StdXor::new(base_port)),
            SingleWidthType::Add => Box::new(StdAdd::new(base_port)),
            SingleWidthType::Sub => Box::new(StdSub::new(base_port)),
            SingleWidthType::Gt => Box::new(StdGt::new(base_port)),
            SingleWidthType::Lt => Box::new(StdLt::new(base_port)),
            SingleWidthType::Eq => Box::new(StdEq::new(base_port)),
            SingleWidthType::Neq => Box::new(StdNeq::new(base_port)),
            SingleWidthType::Ge => Box::new(StdGe::new(base_port)),
            SingleWidthType::Le => Box::new(StdLe::new(base_port)),
            SingleWidthType::Lsh => Box::new(StdLsh::new(base_port, *width)),
            SingleWidthType::Rsh => Box::new(StdRsh::new(base_port, *width)),
            SingleWidthType::Mux => Box::new(StdMux::new(base_port)),
            SingleWidthType::Wire => Box::new(StdWire::new(base_port)),
            SingleWidthType::SignedAdd => Box::new(StdAdd::new(base_port)),
            SingleWidthType::SignedSub => Box::new(StdSub::new(base_port)),
            SingleWidthType::SignedGt => Box::new(StdSgt::new(base_port)),
            SingleWidthType::SignedLt => Box::new(StdSlt::new(base_port)),
            SingleWidthType::SignedEq => Box::new(StdSeq::new(base_port)),
            SingleWidthType::SignedNeq => Box::new(StdSneq::new(base_port)),
            SingleWidthType::SignedGe => Box::new(StdSge::new(base_port)),
            SingleWidthType::SignedLe => Box::new(StdSle::new(base_port)),
            SingleWidthType::SignedLsh => Box::new(StdSlsh::new(base_port)),
            SingleWidthType::SignedRsh => Box::new(StdSrsh::new(base_port)),
            SingleWidthType::MultPipe => {
                Box::new(StdMultPipe::<2>::new(base_port, *width))
            }
            SingleWidthType::SignedMultPipe => {
                // todo: Check if this is actually okay
                Box::new(StdMultPipe::<2>::new(base_port, *width))
            }
            SingleWidthType::DivPipe => {
                Box::new(StdDivPipe::<2, false>::new(base_port, *width))
            }
            SingleWidthType::SignedDivPipe => {
                Box::new(StdDivPipe::<2, true>::new(base_port, *width))
            }
            SingleWidthType::Sqrt => {
                Box::new(Sqrt::<false>::new(base_port, *width, None))
            }
            SingleWidthType::UnsynMult => {
                Box::new(StdUnsynMult::new(base_port, *width))
            }
            SingleWidthType::UnsynDiv => {
                Box::new(StdUnsynDiv::new(base_port, *width))
            }
            SingleWidthType::UnsynMod => {
                Box::new(StdUnsynMod::new(base_port, *width))
            }
            SingleWidthType::UnsynSMult => {
                Box::new(StdUnsynSmult::new(base_port, *width))
            }
            SingleWidthType::UnsynSDiv => {
                Box::new(StdUnsynSdiv::new(base_port, *width))
            }
            SingleWidthType::UnsynSMod => {
                Box::new(StdUnsynSmod::new(base_port, *width))
            }
            SingleWidthType::Undef => {
                Box::new(StdUndef::new(base_port, *width))
            }
        },
        CellPrototype::BitSlice {
            start_idx,
            end_idx,
            out_width,
        } => Box::new(StdBitSlice::new(
            base_port, *start_idx, *end_idx, *out_width,
        )),
        CellPrototype::FixedPoint {
            op,
            width,
            int_width,
            frac_width,
        } => match op {
            FXType::Add | FXType::SignedAdd => Box::new(StdAdd::new(base_port)),
            FXType::Sub | FXType::SignedSub => Box::new(StdSub::new(base_port)),
            FXType::Mult | FXType::SignedMult => Box::new(
                FxpMultPipe::<2>::new(base_port, *int_width, *frac_width),
            ),
            FXType::Div => Box::new(FxpDivPipe::<2, false>::new(
                base_port,
                *int_width,
                *frac_width,
            )),

            FXType::SignedDiv => Box::new(FxpDivPipe::<2, true>::new(
                base_port,
                *int_width,
                *frac_width,
            )),
            FXType::Gt => Box::new(StdGt::new(base_port)),
            FXType::SignedGt => Box::new(StdSgt::new(base_port)),
            FXType::SignedLt => Box::new(StdSlt::new(base_port)),
            FXType::Sqrt => Box::new(Sqrt::<true>::new(
                base_port,
                *width,
                Some(*frac_width),
            )),
        },
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
            is_external: _,
        } => {
            let data = dump.as_ref().and_then(|data| {
                let string = ctx.resolve_id(prim.name);
                data.get_data(string)
            });

            match mem_type {
                MemType::Seq => Box::new(if let Some(data) = data {
                    memories_initialized
                        .insert(ctx.resolve_id(prim.name).clone());
                    SeqMem::new_with_init(base_port, *width, false, dims, data)
                } else {
                    SeqMemD1::new(base_port, *width, false, dims)
                }),
                MemType::Std => Box::new(if let Some(data) = data {
                    memories_initialized
                        .insert(ctx.resolve_id(prim.name).clone());
                    CombMem::new_with_init(base_port, *width, false, dims, data)
                } else {
                    CombMem::new(base_port, *width, false, dims)
                }),
            }
        }

        CellPrototype::Unknown(s, _) => {
            todo!("Primitives {s} not yet implemented")
        }
    }
}
