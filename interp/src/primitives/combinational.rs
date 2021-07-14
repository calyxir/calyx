use super::{stateful::get_param, Primitive};
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
            value: Value::from(value, width).unwrap(),
        }
    }

    pub fn new(params: ir::Binding) -> Self {
        let width = get_param(&params, "WIDTH")
            .expect("Missing width parameter from std_const binding");

        let init_value = get_param(&params, "VALUE")
            .expect("Missing `vale` param from std_const binding");

        let value = Value::from(init_value, width).unwrap();

        Self { value }
    }
}

impl Primitive for StdConst {
    fn is_comb(&self) -> bool {
        true
    }

    fn validate(&self, _inputs: &[(ir::Id, &Value)]) {}

    fn execute(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
        _done_val: Option<&Value>,
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        vec![("out".into(), self.value.clone().into())]
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &Value)],
    ) -> Vec<(ir::Id, crate::values::OutputValue)> {
        vec![("out".into(), self.value.clone().into())]
    }

    fn commit_updates(&mut self) {}

    fn clear_update_buffer(&mut self) {}
}

// ===================== Unary operations ======================
comb_primitive!(StdNot[WIDTH](r#in: WIDTH) -> (out: WIDTH) {
    Value {
        vec: r#in.vec.clone().not(),
    }
    .into()
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
    tr.into()
});
comb_primitive!(StdSub[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    //first turn right into ~right + 1
    let new_right = !right.vec.clone();
    let mut adder = StdAdd::from_constants(WIDTH + 1);
    let new_right = adder
        .execute(
            &[("left".into(), &Value { vec: new_right }),
            ("right".into(), &Value::from(1, WIDTH).unwrap())], None
        )
        .into_iter()
        .next()
        .map(|(_, v)| v)
        .unwrap()
        .unwrap_imm();

    //now do addition. maybe better to use the adder and unwrap the OutputValue?
    let a_iter = left.vec.iter().by_ref();
    let b_iter = new_right.vec.iter().by_ref();
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
    //actually ok if there is overflow from 2sc subtraction (?)
    //have to check if this is ok behavior
    return Value { vec: sum }.into();
    todo!()
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
            let mut index: u64 = 0;
            for bit in r_vec.iter().by_ref() {
                if (index >= 64) & *bit {
                    return Value::zeroes(WIDTH as usize).into();
                }
                index += 1;
            }
        }

        //but that's not the only problem. we can't let [place] get to
        //2^64 or above. However the right value couldn't have even been specified
        //to be greater than or equal to 2^64, because it's constrained by u64.
        //so instead of incrementing [place], just calculate place, but only
        //when [bit] is 1, which can only be true for bits below the 65th (first
        // bit is 2^0)

        let mut index: u32 = 0;
        let mut tr = BitVec::new();
        //first push the requisite # of zeroes
        for bit in right.vec.iter().by_ref() {
            if *bit {
                //not possible for bit to be 1 after the 64th bit
                for _ in 0..u64::pow(2, index) {
                    if tr.len() < WIDTH as usize {
                        tr.push(false);
                    }
                    //no point in appending once we've filled it all with zeroes
                }
            }
            index += 1;
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
        tr.into()
});
comb_primitive!(StdRsh[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    //remove [right] bits from index 0
        //extend to proper size

        //same check as in LSH
        if WIDTH > 64 {
            //check if right is greater than or equal to  2 ^ 64
            let r_vec = &right.vec;
            let mut index: u64 = 0;
            for bit in r_vec.iter().by_ref() {
                if (index >= 64) & *bit {
                    return Value::zeroes(WIDTH as usize).into();
                }
                index += 1;
            }
        }

        let mut index: u32 = 0;
        let mut tr = left.vec.clone();
        //first remove [right] bits
        for bit in right.vec.iter().by_ref() {
            if *bit {
                for _ in 0..u64::pow(2, index) {
                    if tr.len() > 0 {
                        tr.remove(0);
                    }
                }
            }
            index += 1;
        }
        //now resize to proper size, putting 0s at the end (0 is false)
        tr.resize(WIDTH as usize, false);
        let tr = Value { vec: tr };
        assert_eq!(tr.width(), WIDTH);
        //sanity check the widths
        tr.into()
});

// ===================== Logial Operations ======================
comb_primitive!(StdAnd[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Value {
        vec: left.vec.clone() & right.vec.clone(),
    }.into()
});
comb_primitive!(StdOr[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Value {
        vec: left.vec.clone() | right.vec.clone(),
    }.into()
});
comb_primitive!(StdXor[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    Value {
        vec: left.vec.clone() ^ right.vec.clone(),
    }.into()
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

    if tr {
        Value::bit_high().into()
    } else {
        Value::bit_low().into()
    }
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
    if tr {
        Value::bit_high().into()
    } else {
        Value::bit_low().into()
    }
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

    if tr {
        Value::bit_high().into()
    } else {
        Value::bit_low().into()
    }
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
    if tr {
        Value::bit_high().into()
    } else {
        Value::bit_low().into()
    }
});
comb_primitive!(StdEq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();

    //tr represents a = b
    for (ai, bi) in a_iter.zip(b_iter) {
        if !ai & bi || !bi & ai {
            return Value::bit_low().into();
        }
    }

    Value::bit_high().into()
});
comb_primitive!(StdNeq[WIDTH](left: WIDTH, right: WIDTH) -> (out: WIDTH) {
    let a_iter = left.vec.iter().by_ref();
    let b_iter = right.vec.iter().by_ref();

    //tr represents a = b
    for (ai, bi) in a_iter.zip(b_iter) {
        if bi & !ai || !bi & ai {
            return Value::bit_high().into();
        }
    }

    Value::bit_low().into()
});

// ===================== Resizing Operations ======================
comb_primitive!(StdSlice[IN_WIDTH, OUT_WIDTH](r#in: IN_WIDTH) -> (out: OUT_WIDTH) {
    let tr = r#in.clone();
    tr.truncate(OUT_WIDTH as usize).into()
});
comb_primitive!(StdPad[IN_WIDTH, OUT_WIDTH](r#in: IN_WIDTH) -> (out: OUT_WIDTH) {
    let pd = r#in.clone();
    pd.ext(OUT_WIDTH as usize).into()
});
