use crate::flatten::{
    flat_ir::prelude::{AssignedValue, GlobalPortIdx, PortValue},
    primitives::{
        all_defined, comb_primitive, declare_ports, ports,
        prim_trait::UpdateStatus, utils::floored_division, Primitive,
    },
    structures::{
        environment::PortMap,
        index_trait::{IndexRef, SplitIndexRange},
    },
};

use baa::{BitVecOps, BitVecValue};

use super::prim_trait::UpdateResult;

pub struct StdConst {
    value: BitVecValue,
    out: GlobalPortIdx,
}

impl StdConst {
    pub fn new(value: BitVecValue, out: GlobalPortIdx) -> Self {
        Self { value, out }
    }
}

impl Primitive for StdConst {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        Ok(if port_map[self.out].is_undef() {
            // A constant cannot meaningfully be said to belong to a given thread
            port_map[self.out] = PortValue::new_cell(self.value.clone());
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        })
    }

    fn has_comb_path(&self) -> bool {
        true
    }

    fn has_stateful_path(&self) -> bool {
        false
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        SplitIndexRange::new(self.out, self.out, (self.out.index() + 1).into())
    }
}

pub struct StdMux {
    base_port: GlobalPortIdx,
}

impl StdMux {
    declare_ports![ COND: 0, TRU: 1 | FAL:2, OUT: 3 ];
    pub fn new(base: GlobalPortIdx) -> Self {
        Self { base_port: base }
    }
}

impl Primitive for StdMux {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base_port; cond: Self::COND, tru: Self::TRU, fal: Self::FAL, out: Self::OUT];

        let winning_idx =
            port_map[cond].as_bool().map(|c| if c { tru } else { fal });

        if winning_idx.is_some() && port_map[winning_idx.unwrap()].is_def() {
            Ok(port_map.insert_val_general(
                out,
                AssignedValue::cell_value(
                    port_map[winning_idx.unwrap()].val().unwrap().clone(),
                ),
            )?)
        } else {
            port_map.write_undef(out)?;
            Ok(UpdateStatus::Unchanged)
        }
    }

    fn has_stateful_path(&self) -> bool {
        false
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        self.get_signature()
    }
}

comb_primitive!(StdNot(input [0]) -> (out [1]) {
    all_defined!(input);
    Ok(Some(input.not()))
});

comb_primitive!(StdWire(input [0] ) -> (out [1]) {
    Ok(input.val().cloned())
});

// ===================== Unsigned binary operations ======================
comb_primitive!(StdAdd(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.add(right)))
});
comb_primitive!(StdSub(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.sub(right)))
});

comb_primitive!(StdFpAdd(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.add(right)))
});

comb_primitive!(StdFpSub(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.sub(right)))
});

// ===================== Shift Operations ======================
comb_primitive!(StdLsh(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.shift_left(right)))
});

comb_primitive!(StdRsh(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.shift_right(right)))
});

// ===================== Signed Shift Operations ======================
comb_primitive!(StdSlsh(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.shift_left(right)))
});
comb_primitive!(StdSrsh(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.arithmetic_shift_right(right)))
});
// ===================== Logial Operations ======================
comb_primitive!(StdAnd(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.and(right)))
});
comb_primitive!(StdOr(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.or(right)))
});
comb_primitive!(StdXor(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.xor(right)))
});

// ===================== Comparison Operations ======================
comb_primitive!(StdGt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_greater(right).into()))
});
comb_primitive!(StdLt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_less(right).into()))
});
comb_primitive!(StdGe(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_greater_or_equal(right).into()))
});
comb_primitive!(StdLe(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_less_or_equal(right).into()))
});
comb_primitive!(StdEq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_equal(right).into()))
});
comb_primitive!(StdNeq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_not_equal(right).into()))
});

// ===================== Signed Comparison Operations ======================
comb_primitive!(StdSgt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_greater_signed(right).into()))
});
comb_primitive!(StdSlt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_less_signed(right).into()))
});
comb_primitive!(StdSge(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_greater_or_equal_signed(right).into()))
});
comb_primitive!(StdSle(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_less_or_equal_signed(right).into()))
});
comb_primitive!(StdSeq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_equal(right).into()))
});
comb_primitive!(StdSneq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_not_equal(right).into()))
});

// ===================== Unsigned FP Comparison Operators ======================
comb_primitive!(StdFpGt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_greater(right).into()))
});

// ===================== Signed FP Comparison Operators ======================
comb_primitive!(StdFpSgt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_greater_signed(right).into()))
});

comb_primitive!(StdFpSlt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.is_less_signed(right).into()))
});

// ===================== Resizing Operations ======================
comb_primitive!(StdSlice[OUT_WIDTH](input [0]) -> (out [1]) {
    all_defined!(input);
    let msb = OUT_WIDTH - 1;
    Ok(Some(input.slice(msb, 0)))
});
comb_primitive!(StdPad[OUT_WIDTH](input [0]) -> (out [1]) {
    all_defined!(input);
    let by = OUT_WIDTH - input.width();
    Ok(Some(input.zero_extend(by)))
});

comb_primitive!(StdCat(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.concat(right)))
});

comb_primitive!(StdBitSlice[START_IDX, END_IDX](input [0]) -> (out [1]) {
    all_defined!(input);
    let (msb, lsb) = (END_IDX, START_IDX);
    Ok(Some(input.slice(msb, lsb)))
});

// ===================== Unsynthesizeable Operations ======================
comb_primitive!(StdUnsynMult(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    Ok(Some(left.mul(right)))
});

comb_primitive!(StdUnsynDiv[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    let res = left.to_big_uint() / right.to_big_uint();
    Ok(Some(BitVecValue::from_big_uint(&res, WIDTH)))
});

comb_primitive!(StdUnsynSmult(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    // FIXME: is there a difference for signed?
    Ok(Some(left.mul(right)))
});

comb_primitive!(StdUnsynSdiv[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    let res = left.to_big_int() / right.to_big_int();
    Ok(Some(BitVecValue::from_big_int(&res, WIDTH)))
});

comb_primitive!(StdUnsynMod[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    let res = left.to_big_uint() % right.to_big_uint();
    Ok(Some(BitVecValue::from_big_uint(&res, WIDTH)))
});

comb_primitive!(StdUnsynSmod[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    let res = left.to_big_int() - right.to_big_int() * floored_division(
            &left.to_big_int(),
            &right.to_big_int());
    Ok(Some(BitVecValue::from_big_int(&res, WIDTH)))
});

pub struct StdUndef(GlobalPortIdx);

impl StdUndef {
    pub fn new(base_port: GlobalPortIdx, _width: u32) -> Self {
        Self(base_port)
    }
}

impl Primitive for StdUndef {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        port_map.write_undef(self.0)?;
        Ok(UpdateStatus::Unchanged)
    }

    fn get_ports(&self) -> SplitIndexRange<GlobalPortIdx> {
        SplitIndexRange::new(self.0, self.0, (self.0.index() + 1).into())
    }
}
