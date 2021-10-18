use super::{
    super::errors::InterpreterResult,
    prim_utils::{get_input_unwrap, get_param},
    Primitive,
};
use crate::comb_primitive;
use crate::values::Value;
use bitvec::vec::BitVec;
use calyx::ir;
use std::ops::Not;

/// A constant.
#[derive(Default, Debug)]
pub struct StdConst {
    value: Value,
}

impl StdConst {
    pub fn from_constants(value: u64, width: u64) -> Self {
        StdConst {
            value: Value::from(value, width),
        }
    }

    pub fn new(params: &ir::Binding) -> Self {
        let width = get_param(params, "WIDTH")
            .expect("Missing width parameter from std_const binding");

        let init_value = get_param(params, "VALUE")
            .expect("Missing `vale` param from std_const binding");

        let value = Value::from(init_value, width);

        Self { value }
    }
}

impl Primitive for StdConst {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(vec![])
    }

    fn is_comb(&self) -> bool {
        true
    }

    fn validate(&self, _inputs: &[(ir::Id, &Value)]) {}

    fn execute(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(vec![("out".into(), self.value.clone())])
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(vec![("out".into(), self.value.clone())])
    }
}

// ===================== New core ======================

pub struct StdMux {
    width: u64,
}

impl StdMux {
    pub fn new(params: &ir::Binding) -> Self {
        let width = get_param(params, "WIDTH")
            .expect("Missing width parameter from std_const binding");

        Self { width }
    }
}

impl Primitive for StdMux {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(vec![])
    }

    fn is_comb(&self) -> bool {
        true
    }

    fn validate(&self, inputs: &[(ir::Id, &Value)]) {
        for (id, v) in inputs {
            match id.as_ref() {
                "tru" => assert_eq!(v.len() as u64, self.width),
                "fal" => assert_eq!(v.len() as u64, self.width),
                "cond" => assert_eq!(v.len() as u64, 1),
                p => unreachable!("Unknown port: {}", p),
            }
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let cond = get_input_unwrap(inputs, "cond");
        let tru = get_input_unwrap(inputs, "tru");
        let fal = get_input_unwrap(inputs, "fal");

        let out = if cond.as_bool() { tru } else { fal };
        Ok(vec![("out".into(), out.clone())])
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        Ok(vec![("out".into(), Value::zeroes(self.width))])
    }
}

// ===================== Unary operations ======================
comb_primitive!(StdNot[WIDTH](r#in: WIDTH) -> (out: WIDTH) {
    Ok(Value {
        vec: r#in.vec.clone().not(),
    })
});

// ===================== Unsigned binary operations ======================
comb_primitive!(StdAdd[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();
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
    let tr = Value { vec: sum };
    //as a sanity check, check tr has same width as left
    assert_eq!(tr.width(), left.width());
    Ok(tr)
});
comb_primitive!(StdSub[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    //first turn right into ~right + 1
    let new_right = !right.vec.clone();
    let mut adder = StdAdd::from_constants(WIDTH);
    let (_,new_right) = adder
        .execute(
            &[("left".into(), &Value { vec: new_right }),
            ("right".into(), &Value::from(1, WIDTH))],
        )?
        .into_iter()
        .next()
        .unwrap();
    //then add left and new_right
    Ok(adder.execute(&[("left".into(), left),
    ("right".into(), &new_right)])?.into_iter().next().map(|(_, v)| v).unwrap())
});

// TODO (Griffin): Make these wrappers around the normal add
comb_primitive!(StdFpAdd[WIDTH, INT_WIDTH, FRAC_WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();
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
    let tr = Value { vec: sum };
    //as a sanity check, check tr has same width as left
    assert_eq!(tr.width(), left.width());
    Ok(tr)
});
comb_primitive!(StdFpSub[WIDTH, INT_WIDTH, FRAC_WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    //first turn right into ~right + 1
    let new_right = !right.vec.clone();
    let mut adder = StdAdd::from_constants(WIDTH);
    let new_right = adder
        .execute(
            &[("left".into(), &Value { vec: new_right }),
            ("right".into(), &Value::from(1, WIDTH))],
        )?
        .into_iter()
        .next()
        .map(|(_, v)| v)
        .unwrap();
    //then add left and new_right
    let out = adder.execute(&[("left".into(), left),
    ("right".into(), &new_right)])?.into_iter().next().map(|(_, v)| v).unwrap();

    Ok(out)
});

// ===================== Shift Operations ======================
comb_primitive!(StdLsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    //to avoid the casting overflow,
        //we know that [left], [right], and [self]
        //are capped at bitwidths as large as largest u64 (2^64 - 1 = 1.84 * 10^19 ...)
        //so, if [self] has a width greater than 64,
        //and the 65th index is a 1, we just automatically return a 0 of the
        //appropriate bitwidth!

        if WIDTH > 64 {
            //check if right is greater than or equal to  2 ^ 64
            let r_vec = &right.vec;
            for bit in r_vec.iter().by_ref().skip(64) {
                if *bit {
                    return Ok(Value::zeroes(WIDTH as usize));
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
        for (index, bit) in right.vec.iter().by_ref().enumerate() {
            if *bit {
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
        let mut to_append = left.clone().vec;
        tr.append(&mut to_append);
        tr.truncate(WIDTH as usize);
        let tr = Value { vec: tr };
        assert_eq!(tr.width(), WIDTH);
        //sanity check the widths
        Ok(tr)
});
comb_primitive!(StdRsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    //remove [right] bits from index 0
        //extend to proper size

        //same check as in LSH
        if WIDTH > 64 {
            //check if right is greater than or equal to  2 ^ 64
            let r_vec = &right.vec;
            for bit in r_vec.iter().by_ref().skip(64) {
                if *bit {
                    return Ok(Value::zeroes(WIDTH as usize));
                }
            }
        }

        let mut tr = left.vec.clone();
        //first remove [right] bits
        for (index, bit) in right.vec.iter().by_ref().enumerate() {
            if *bit {
                for _ in 0..u64::pow(2, index as u32) {
                    if !tr.is_empty() {
                        tr.remove(0);
                    }
                }
            }
        }
        //now resize to proper size, putting 0s at the end (0 is false)
        tr.resize(WIDTH as usize, false);
        let tr = Value { vec: tr };
        assert_eq!(tr.width(), WIDTH);
        //sanity check the widths
        Ok(tr)
});

// ===================== Signed Shift Operations ======================
comb_primitive!(StdSlsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let shift_amount = right.as_usize();
    let mut val = left.clone().vec;
    val.shift_right(shift_amount);
    Ok(Value {vec: val })

});
comb_primitive!(StdSrsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let shift_amount = right.as_usize();
    let sign: bool = left.vec[left.vec.len()-1]; //msb
    let mut val = left.vec.clone();
    val.shift_left(shift_amount);
    if sign {
        for mut bit in val.iter_mut().rev().take(shift_amount) {
            *bit = true;
        }
    }
    Ok(Value { vec: val })
});
// ===================== Logial Operations ======================
comb_primitive!(StdAnd[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(Value {
        vec: left.vec.clone() & right.vec.clone(),
    })
});
comb_primitive!(StdOr[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(Value {
        vec: left.vec.clone() | right.vec.clone(),
    })
});
comb_primitive!(StdXor[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(Value {
        vec: left.vec.clone() ^ right.vec.clone(),
    })
});

// ===================== Comparison Operations ======================
comb_primitive!(StdGt[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();
    let mut tr = false;

    //as we proceed up in magnitude, it doesn't matter which port was less
    //b/c [100....000] > [011....111] always.
    //but if ai = bi, it matters which was higher previously
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = ai & !bi || tr & !bi || tr & ai;
    }

    Ok(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdLt[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();
    let mut tr = false;

    //tr represents a < b
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = !ai & bi | tr & !ai | tr & bi;
    }

    //same as gt, just reverse the if.
    //but actually not so if they are equal... should change the loop
    Ok(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdGe[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();
    let mut tr = true; //diff between gt and ge is just assume they r equal

    //as we proceed up in magnitude, it doesn't matter which port was less
    //b/c [100....000] > [011....111] always.
    //but if ai = bi, it matters which was higher previously
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = ai & !bi || tr & !bi || tr & ai;
    }

    Ok(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdLe[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();
    let mut tr = true; //diff between le and lt is just assume they are equal

    //tr represents a <= b
    for (ai, bi) in a_iter.zip(b_iter) {
        tr = !ai & bi | tr & !ai | tr & bi;
    }

    //same as gt, just reverse the if.
    //but actually not so if they are equal... should change the loop
    Ok(if tr {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdEq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();

    //tr represents a = b
    for (ai, bi) in a_iter.zip(b_iter) {
        if !ai & bi || !bi & ai {
            return Ok(Value::bit_low());
        }
    }

    Ok(Value::bit_high())
});
comb_primitive!(StdNeq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();

    //tr represents a = b
    for (ai, bi) in a_iter.zip(b_iter) {
        if bi & !ai || !bi & ai {
            return Ok(Value::bit_high());
        }
    }

    Ok(Value::bit_low())
});
// TODO (griffin) : replace these comparsions with bit-aware variants
// ===================== Signed Comparison Operations ======================
comb_primitive!(StdSgt[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() > right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdSlt[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() < right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdSge[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() >= right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdSle[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() <= right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdSeq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() == right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});
comb_primitive!(StdSneq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() != right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});

// ===================== Unsigned FP Comparison Operators ======================
comb_primitive!(StdFpGt[WIDTH, INT_WIDTH, FRAC_WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_u128() > right.as_u128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});

// ===================== Signed FP Comparison Operators ======================
comb_primitive!(StdFpSgt[WIDTH, INT_WIDTH, FRAC_WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() > right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});

comb_primitive!(StdFpSlt[WIDTH, INT_WIDTH, FRAC_WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Ok(if left.as_i128() < right.as_i128() {
        Value::bit_high()
    } else {
        Value::bit_low()
    })
});

// ===================== Resizing Operations ======================
comb_primitive!(StdSlice[IN_WIDTH, OUT_WIDTH](r#in: IN_WIDTH) -> (out: OUT_WIDTH) {
    Ok(r#in.truncate(OUT_WIDTH as usize))
});
comb_primitive!(StdPad[IN_WIDTH, OUT_WIDTH](r#in: IN_WIDTH) -> (out: OUT_WIDTH) {
    Ok(r#in.ext(OUT_WIDTH as usize))
});
