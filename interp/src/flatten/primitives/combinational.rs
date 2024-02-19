use std::ops::Not;

use bitvec::vec::BitVec;

use crate::{
    flatten::{
        flat_ir::prelude::{AssignedValue, GlobalPortIdx, PortValue},
        primitives::{
            all_defined, comb_primitive, declare_ports, ports,
            prim_trait::UpdateStatus, Primitive,
        },
        structures::environment::PortMap,
    },
    primitives::stateful::floored_division,
    values::Value,
};

use super::prim_trait::UpdateResult;

pub struct StdConst {
    value: Value,
    out: GlobalPortIdx,
}

impl StdConst {
    pub fn new(value: Value, out: GlobalPortIdx) -> Self {
        Self { value, out }
    }
}

impl Primitive for StdConst {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        Ok(if port_map[self.out].is_undef() {
            port_map[self.out] = PortValue::new_cell(self.value.clone());
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        })
    }

    fn exec_cycle(&mut self, _port_map: &mut PortMap) -> UpdateResult {
        Ok(UpdateStatus::Unchanged)
    }

    fn has_comb(&self) -> bool {
        false
    }

    fn has_stateful(&self) -> bool {
        false
    }
}

pub struct StdMux {
    base: GlobalPortIdx,
}

impl StdMux {
    declare_ports![ COND: 0, TRU: 1, FAL:2, OUT: 3];
    pub fn new(base: GlobalPortIdx) -> Self {
        Self { base }
    }
}

impl Primitive for StdMux {
    fn exec_comb(&self, port_map: &mut PortMap) -> UpdateResult {
        ports![&self.base; cond: Self::COND, tru: Self::TRU, fal: Self::FAL, out: Self::OUT];

        let winning_idx =
            port_map[cond].as_bool().map(|c| if c { tru } else { fal });

        if winning_idx.is_some() && port_map[winning_idx.unwrap()].is_def() {
            Ok(port_map.insert_val(
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

    fn has_stateful(&self) -> bool {
        false
    }
}

comb_primitive!(StdNot(input [0]) -> (out [1]) {
    all_defined!(input);
    Ok(Some(input.clone_bit_vec().not().into()))
});

comb_primitive!(StdWire(input [0] ) -> (out [1]) {
    Ok(input.val().cloned())
});

// ===================== Unsigned binary operations ======================
comb_primitive!(StdAdd(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();
    let mut c_in = false;
    let mut sum = BitVec::new();
    for (ai, bi) in a_iter.zip(b_iter) {
        sum.push(
            c_in & !ai & !bi
                || bi & !c_in & !ai
                || ai & !c_in & !bi
                || ai & bi & c_in,
        );
        c_in = bi & c_in || ai & c_in || ai & bi || ai & c_in & bi;
    }

    let tr: Value = sum.into();
    //as a sanity check, check tr has same width as left
    debug_assert_eq!(tr.width(), left.width());
    Ok(Some(tr))
});
comb_primitive!(StdSub(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    // TODO griffin: the old approach is not possible with the way primitives work
    let result = Value::from(left.as_unsigned() - right.as_unsigned(), left.width());
    Ok(Some(result))
});

// TODO (Griffin): Make these wrappers around the normal add
comb_primitive!(StdFpAdd(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    let a_iter = left.iter();
    let b_iter = right.iter();
    let mut c_in = false;
    let mut sum = BitVec::new();
    for (ai, bi) in a_iter.zip(b_iter) {
        sum.push(
            c_in & !ai & !bi
                || bi & !c_in & !ai
                || ai & !c_in & !bi
                || ai & bi & c_in,
        );
        c_in = bi & c_in || ai & c_in || ai & bi || ai & c_in & bi;
    }
    let tr = Value::from_bv(sum);

    //as a sanity check, check tr has same width as left
    debug_assert_eq!(tr.width(), left.width());
    Ok(Some(tr))
});

comb_primitive!(StdFpSub(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    let result = Value::from(left.as_unsigned() - right.as_unsigned(), left.width());

    Ok(Some(result))
});

// ===================== Shift Operations ======================
comb_primitive!(StdLsh[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);
    //to avoid the casting overflow,
        //we know that [left], [right], and [self]
        //are capped at bitwidths as large as largest u64 (2^64 - 1 = 1.84 * 10^19 ...)
        //so, if [self] has a width greater than 64,
        //and the 65th index is a 1, we just automatically return a 0 of the
        //appropriate bitwidth!

        if WIDTH > 64 {
            //check if right is greater than or equal to  2 ^ 64

            for bit in right.iter().by_ref().skip(64) {
                if bit {
                    return Ok( Some(Value::zeroes(WIDTH as usize)));
                }
            }
        }

        //but that's not the only problem. we can't let [place] get to
        //2^64 or above. However the right value couldn't have even been specified
        //to be greater than or equal to 2^64, because it's constrained by u64.
        //so instead of incrementing [place], just calculate place, but only
        //when [bit] is 1, which can only be true for bits below the 65th (first
        // bit is 2^0)

        let mut tr = BitVec::new();
        //first push the requisite # of zeroes
        for (index, bit) in right.iter().enumerate() {
            if bit {
                //not possible for bit to be 1 after the 64th bit
                for _ in 0..u64::pow(2, index as u32) {
                    if tr.len() < WIDTH as usize {
                        tr.push(false);
                    }
                    //no point in appending once we've filled it all with zeroes
                }
            }
        }
        //then copy over the bits from [left] onto the back (higher-place bits) of
        //[tr]. Then truncate, aka slicing off the bits that exceed the width of this
        //component
        let mut to_append = left.clone_bit_vec();
        tr.append(&mut to_append);
        tr.truncate(WIDTH as usize);
        let tr = Value::from_bv(tr);
        debug_assert_eq!(tr.width(), WIDTH as u64);
        //sanity check the widths
        Ok(Some(tr))
});

comb_primitive!(StdRsh[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    //remove [right] bits from index 0
        //extend to proper size

        //same check as in LSH
        if WIDTH > 64 {
            //check if right is greater than or equal to  2 ^ 64
            for bit in right.iter().skip(64) {
                if bit {
                    return Ok( Some(Value::zeroes(WIDTH as usize)));
                }
            }
        }

        let mut tr = left.clone_bit_vec();
        //first remove [right] bits
        for (index, bit) in right.iter().enumerate() {
            if bit {
                for _ in 0..u64::pow(2, index as u32) {
                    if !tr.is_empty() {
                        tr.remove(0);
                    }
                }
            }
        }
        //now resize to proper size, putting 0s at the end (0 is false)
        tr.resize(WIDTH as usize, false);
        let tr = Value::from_bv(tr);
        debug_assert_eq!(tr.width(), WIDTH as u64);
        //sanity check the widths
        Ok(Some(tr))
});

// ===================== Signed Shift Operations ======================
comb_primitive!(StdSlsh(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let shift_amount = right.as_usize();
    let mut val = left.clone_bit_vec();
    val.shift_right(shift_amount);
    let result: Value = val.into();
    Ok(Some(result))

});
comb_primitive!(StdSrsh(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let shift_amount = right.as_usize();
    let sign: bool = left[left.len()-1]; //msb
    let mut val = left.clone_bit_vec();
    val.shift_left(shift_amount);
    if sign {
        for mut bit in val.iter_mut().rev().take(shift_amount) {
            *bit = true;
        }
    }
    let result: Value = val.into();
    Ok(Some(result))
});
// ===================== Logial Operations ======================
comb_primitive!(StdAnd(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let result: Value = (left.clone_bit_vec() & right.clone_bit_vec()).into();
    Ok(Some(result))
});
comb_primitive!(StdOr(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let result: Value = (left.clone_bit_vec() | right.clone_bit_vec()).into();
    Ok(Some(result))
});
comb_primitive!(StdXor(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let result: Value = (left.clone_bit_vec() ^ right.clone_bit_vec()).into();
    Ok(Some(result))
});

// ===================== Comparison Operations ======================
comb_primitive!(StdGt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();
    let mut tr = false;

    //as we proceed up in magnitude, it doesn't matter which port was less
    //b/c [100....000] > [011....111] always.
    //but if ai = bi, it matters which was higher previously
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = ai & !bi || tr & !bi || tr & ai;
    }

    Ok(Some(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdLt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();
    let mut tr = false;

    //tr represents a < b
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = !ai & bi | tr & !ai | tr & bi;
    }

    //same as gt, just reverse the if.
    //but actually not so if they are equal... should change the loop
    Ok(Some(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdGe(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();
    let mut tr = true; //diff between gt and ge is just assume they r equal

    //as we proceed up in magnitude, it doesn't matter which port was less
    //b/c [100....000] > [011....111] always.
    //but if ai = bi, it matters which was higher previously
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = ai & !bi || tr & !bi || tr & ai;
    }

    Ok(Some(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdLe(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();
    let mut tr = true; //diff between le and lt is just assume they are equal

    //tr represents a <= b
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = !ai & bi | tr & !ai | tr & bi;
    }

    //same as gt, just reverse the if.
    //but actually not so if they are equal... should change the loop
    Ok(Some(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdEq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();

    //tr represents a = b
    for (ai, bi) in a_iter.zip(b_iter) {
        if !ai & bi || !bi & ai {
            return Ok(Some(Value::bit_low()));
        }
    }

    Ok(Some(Value::bit_high()))
});
comb_primitive!(StdNeq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    let a_iter = left.iter();
    let b_iter = right.iter();

    //tr represents a = b
    for (ai, bi) in a_iter.zip(b_iter) {
        if bi & !ai || !bi & ai {
            return Ok(Some(Value::bit_high()));
        }
    }

    Ok(Some(Value::bit_low()))
});

// ===================== Signed Comparison Operations ======================
comb_primitive!(StdSgt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() > right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdSlt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() < right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdSge(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() >= right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdSle(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() <= right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdSeq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() == right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});
comb_primitive!(StdSneq(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() != right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});

// ===================== Unsigned FP Comparison Operators ======================
comb_primitive!(StdFpGt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok(
        Some(if left.as_unsigned() > right.as_unsigned() {
            Value::bit_high()
        } else {
            Value::bit_low()
        })
    )
});

// ===================== Signed FP Comparison Operators ======================
comb_primitive!(StdFpSgt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() > right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});

comb_primitive!(StdFpSlt(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(if left.as_signed() < right.as_signed() {
        Value::bit_high()
    } else {
        Value::bit_low()
    }))
});

// ===================== Resizing Operations ======================
comb_primitive!(StdSlice[OUT_WIDTH](input [0]) -> (out [1]) {
    all_defined!(input);

    Ok( Some(input.truncate(OUT_WIDTH as usize)))
});
comb_primitive!(StdPad[OUT_WIDTH](input [0]) -> (out [1]) {
    all_defined!(input);

    Ok( Some(input.ext(OUT_WIDTH as usize)))
});

comb_primitive!(StdCat(left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok(Some(Value::concat(left, right)))
});

// ===================== Unsynthesizeable Operations ======================
comb_primitive!(StdUnsynMult[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(Value::from(left.as_unsigned() * right.as_unsigned(), WIDTH)))
});

comb_primitive!(StdUnsynDiv[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(Value::from(left.as_unsigned() / right.as_unsigned(), WIDTH)))
});

comb_primitive!(StdUnsynSmult[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(Value::from(left.as_signed() * right.as_signed(), WIDTH)))
});

comb_primitive!(StdUnsynSdiv[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(Value::from(left.as_signed() / right.as_signed(), WIDTH)))
});

comb_primitive!(StdUnsynMod[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(Value::from(left.as_unsigned() % right.as_unsigned(), WIDTH)))
});

comb_primitive!(StdUnsynSmod[WIDTH](left [0], right [1]) -> (out [2]) {
    all_defined!(left, right);

    Ok( Some(Value::from(left.as_signed() - right.as_signed() * floored_division(
            &left.as_signed(),
            &right.as_signed()), WIDTH)))
});
