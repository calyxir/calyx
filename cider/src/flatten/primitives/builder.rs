use ahash::{HashMap, HashSet};

use super::{
    Primitive, combinational::*, prim_trait::RaceDetectionPrimitive,
    stateful::*,
};
use crate::{
    flatten::{
        flat_ir::{
            cell_prototype::{
                CellPrototype, DoubleWidthType, FXType, MemType,
                MemoryPrototype, SingleWidthType, TripleWidthType,
            },
            indexes::{CellDefinitionIdx, GlobalCellIdx, MemoryRegion},
            prelude::GlobalPortIdx,
        },
        structures::{
            context::Context,
            environment::{CellLedger, MemoryMap, clock::ClockMap},
        },
    },
    serialization::DataDump,
};

use baa::BitVecValue;

#[allow(clippy::too_many_arguments)]
pub fn build_primitive(
    prim_idx: CellDefinitionIdx,
    base_port: GlobalPortIdx,
    // the global idx of the instantiated primitive
    cell_idx: GlobalCellIdx,
    // extras for memory initialization
    ctx: &Context,
    dump: &Option<DataDump>,
    memories_initialized: &mut HashSet<String>,
    // if the clock map is not provided then data race checking is disabled
    mut clocks: Option<&mut ClockMap>,
    state_map: &mut MemoryMap,
    entangle_map: &mut HashMap<CellDefinitionIdx, MemoryRegion>,
) -> CellLedger {
    let prim = &ctx.secondary.local_cell_defs[prim_idx];

    let b: Box<dyn Primitive> = match &prim.prototype {
        CellPrototype::Constant {
            value: val,
            width,
            c_type: _,
        } => {
            let v = BitVecValue::from_u64(*val, *width);
            Box::new(StdConst::new(v, base_port, state_map))
        }

        CellPrototype::Component(_) => unreachable!(
            "Build primitive erroneously called on a calyx component"
        ),
        CellPrototype::SingleWidth { op, width } => {
            match op {
                SingleWidthType::Reg => {
                    let b = StdReg::new(
                        base_port,
                        cell_idx,
                        *width,
                        &mut clocks,
                        state_map,
                    );
                    if clocks.is_some() {
                        let b: Box<dyn RaceDetectionPrimitive> = Box::new(b);
                        return b.into();
                    } else {
                        let b: Box<dyn Primitive> = Box::new(b);
                        return b.into();
                    }
                }
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
                SingleWidthType::Lsh => Box::new(StdLsh::new(base_port)),
                SingleWidthType::Rsh => Box::new(StdRsh::new(base_port)),
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
                    Box::new(StdUnsynMult::new(base_port))
                }
                SingleWidthType::UnsynDiv => {
                    Box::new(StdUnsynDiv::new(base_port, *width))
                }
                SingleWidthType::UnsynMod => {
                    Box::new(StdUnsynMod::new(base_port, *width))
                }
                SingleWidthType::UnsynSMult => {
                    Box::new(StdUnsynSmult::new(base_port))
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
                SingleWidthType::UnsynAssert => {
                    todo!()
                }
            }
        }
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
        CellPrototype::DoubleWidth { op, width2, .. } => match op {
            DoubleWidthType::Slice => {
                Box::new(StdSlice::new(base_port, *width2))
            }
            DoubleWidthType::Pad => Box::new(StdPad::new(base_port, *width2)),
        },
        CellPrototype::TripleWidth {
            op, width1, width2, ..
        } => match op {
            TripleWidthType::Cat => {
                // Turns out under the assumption that the primitive is well formed,
                // none of these parameter values are actually needed
                Box::new(StdCat::new(base_port))
            }
            TripleWidthType::BitSlice => {
                Box::new(StdBitSlice::new(base_port, *width1, *width2))
            }
        },

        CellPrototype::Memory(MemoryPrototype {
            mem_type,
            width,
            dims,
            is_external: _,
        }) => {
            let config =
                MemConfigInfo::new(base_port, cell_idx, *width, false, dims);

            let data = dump.as_ref().and_then(|data| {
                let string = ctx.resolve_id(prim.name);
                data.get_data(string)
            });

            let merge_set = ctx
                .secondary
                .entangled_mems
                .iter()
                .find(|group| group.contains(prim_idx));

            match mem_type {
                MemType::Seq => {
                    if let Some(set) = merge_set {
                        if let Some(region) =
                            entangle_map.get(&set.representative())
                        {
                            let mem = SeqMem::new_with_region(config, *region);
                            return box_race_detection_primitive(
                                clocks.is_some(),
                                mem,
                            );
                        }
                    }
                    let region_start = state_map.peek_next_memory_location();

                    let mem = if let Some(data) = data {
                        memories_initialized
                            .insert(ctx.resolve_id(prim.name).clone());
                        SeqMem::new_with_init(
                            config,
                            data,
                            &mut clocks,
                            state_map,
                        )
                    } else {
                        SeqMemD1::new(config, &mut clocks, state_map)
                    };

                    if let Some(set) = merge_set {
                        // the early return from the prior section means that we
                        // must be the first memory seen for the entangled group
                        entangle_map.insert(
                            set.representative(),
                            MemoryRegion::new(
                                region_start,
                                state_map.peek_next_memory_location(),
                            ),
                        );
                    }

                    return box_race_detection_primitive(clocks.is_some(), mem);
                }
                MemType::Std => {
                    if let Some(set) = merge_set {
                        if let Some(region) =
                            entangle_map.get(&set.representative())
                        {
                            let mem = CombMem::new_with_region(config, *region);
                            return box_race_detection_primitive(
                                clocks.is_some(),
                                mem,
                            );
                        }
                    }
                    let region_start = state_map.peek_next_memory_location();

                    let mem = if let Some(data) = data {
                        memories_initialized
                            .insert(ctx.resolve_id(prim.name).clone());
                        CombMem::new_with_init(
                            config,
                            data,
                            &mut clocks,
                            state_map,
                        )
                    } else {
                        CombMem::new(config, &mut clocks, state_map)
                    };

                    if let Some(set) = merge_set {
                        // the early return from the prior section means that we
                        // must be the first memory seen for the entangled group
                        entangle_map.insert(
                            set.representative(),
                            MemoryRegion::new(
                                region_start,
                                state_map.peek_next_memory_location(),
                            ),
                        );
                    }

                    return box_race_detection_primitive(clocks.is_some(), mem);
                }
            }
        }

        CellPrototype::Unknown(s, _) => {
            todo!("Primitives {s} not yet implemented")
        }
    };
    b.into()
}

fn box_race_detection_primitive<T: RaceDetectionPrimitive + 'static>(
    race_detection_enabled: bool,
    prim: T,
) -> CellLedger {
    if race_detection_enabled {
        let b: Box<dyn RaceDetectionPrimitive> = Box::new(prim);
        b.into()
    } else {
        let b: Box<dyn Primitive> = Box::new(prim);
        b.into()
    }
}
