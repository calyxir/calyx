#[cfg(test)]
#[cfg(test)]
mod prim_test {
    use crate::primitives::*;
    use crate::values::Value;
    #[test]
    fn test_std_reg() {
        //try out reading and writing to a register, and coordinating its read and write signals
        //remember that registers are mutable, not functional

        //try loading a register with a value of the right size, then too small, then too big
        let val = Value::try_from_init(16, 6).unwrap();
        let mut reg1 = StdReg::new(6);
        reg1.set_write_en_high();
        reg1.load_value(val);
        reg1.set_write_en_low();
        reg1.set_done_high();
        assert_eq!(reg1.read_value().as_u64(), 16);
        reg1.set_write_en_high();
        reg1.load_value(Value::try_from_init(32, 6).unwrap());
        reg1.set_write_en_low();
        reg1.set_done_high();
        assert_eq!(reg1.read_u64(), 32);
        //same register, try loading while write en is low
        reg1.load_value(Value::try_from_init(15, 6).unwrap());
        assert_eq!(reg1.read_u64(), 32);
    }
    #[test]
    #[should_panic]
    fn reg_too_big() {
        let mut reg1 = StdReg::new(5);
        //now try loading in a value that is too big(??)
        let val = Value::try_from_init(32, 6).unwrap();
        reg1.set_write_en_high();
        reg1.load_value(val); //panic here pls
        reg1.set_write_en_low();
        reg1.set_done_high();
    }
    #[test]
    fn test_std_const() {
        let const_31 = StdConst::new_from_u64(5, 31);
        assert_eq!(const_31.read_val().as_u64(), 31); //can rust check this equality?
        assert_eq!(const_31.read_u64(), 31);
        let val_31 = Value::try_from_init(31, 5).unwrap();
        let const_31 = StdConst::new(5, val_31);
        assert_eq!(const_31.read_val().as_u64(), 31);
        assert_eq!(const_31.read_u64(), 31);
    }
    #[test]
    fn test_std_lsh() {
        let left = Value::try_from_init(31, 5).unwrap();
        let right = Value::try_from_init(2, 5).unwrap();
        let lsh = StdLsh::new(5);
        let out = lsh.execute_bin(&left, &right);
        println!("lsh of 31 by 2: {}", out);
        assert_eq!(out.as_u64(), 28);
        //make a Value with bitwidth >= # of bits in binnum of given u64
        let left = Value::try_from_init(15, 4).unwrap(); //15 is [1111] -> [1100] which is 12
        let right = Value::try_from_init(2, 4).unwrap();
        let lsh = StdLsh::new(4);
        let out = lsh.execute_bin(&left, &right);
        println!("lsh of 15 by 2: {}", out);
        assert_eq!(out.as_u64(), 12);
    }
    #[test]
    fn test_std_rsh() {
        let left = Value::try_from_init(15, 4).unwrap();
        let right = Value::try_from_init(2, 4).unwrap();
        let rsh = StdRsh::new(4);
        let out = rsh.execute_bin(&left, &right);
        assert_eq!(out.as_u64(), 3);
    }
    #[test]
    fn test_std_add() {
        let add0 = Value::try_from_init(3, 4).unwrap();
        let add1 = Value::try_from_init(10, 4).unwrap();
        let add = StdAdd::new(4);
        let res_add = add.execute_bin(&add0, &add1);
        assert_eq!(res_add.as_u64(), 13);
    }
    #[test]
    fn test_std_sub() {
        let sub0 = Value::try_from_init(10, 4).unwrap();
        let sub1 = Value::try_from_init(6, 4).unwrap();
        let sub = StdSub::new(4);
        let res_sub = sub.execute_bin(&sub0, &sub1);
        assert_eq!(res_sub.as_u64(), 4);
    }
    #[test]
    fn test_std_slice() {
        //101 in binary is [1100101], take first 4 bits -> [0101] = 5
        let to_slice = Value::try_from_init(101, 7).unwrap();
        let std_slice = StdSlice::new(7, 4);
        let res_slice = std_slice.execute_unary(&to_slice); //note that once we implement execute_unary, have to change this
        assert_eq!(res_slice.as_u64(), 5);
    }
    #[test]
    fn test_std_pad() {
        let to_pad = Value::try_from_init(101, 7).unwrap();
        let std_pad = StdPad::new(7, 9);
        let res_pad = std_pad.execute_unary(&to_pad);
        assert_eq!(res_pad.as_u64(), 101);
    }
    /// Logical Operators
    #[test]
    fn test_std_not() {
        let not0 = Value::try_from_init(10, 4).unwrap();
        let std_not = StdNot::new(4);
        let res_not = std_not.execute_unary(&not0);
        assert_eq!(res_not.as_u64(), 5);
    }
    #[test]
    fn test_std_and() {
        //101: [1100101], 78: [1001110] & -> [1000100] which is 68
        let and0 = Value::try_from_init(101, 7).unwrap();
        let and1 = Value::try_from_init(78, 7).unwrap();
        let std_and = StdAnd::new(7);
        let res_and = std_and.execute_bin(&and0, &and1);
        assert_eq!(res_and.as_u64(), 68);
        // Test for mismatch in widths?
    }
    #[test]
    fn test_std_or() {
        let or0 = Value::try_from_init(5, 3).unwrap();
        let or1 = Value::try_from_init(3, 3).unwrap();
        let std_or = StdOr::new(3);
        let res_or = std_or.execute_bin(&or0, &or1);
        assert_eq!(res_or.as_u64(), 7);
    }
    #[test]
    fn test_std_xor() {
        let xor0 = Value::try_from_init(5, 3).unwrap();
        let xor1 = Value::try_from_init(3, 3).unwrap();
        let std_xor = StdXor::new(3);
        let res_xor = std_xor.execute_bin(&xor0, &xor1);
        assert_eq!(res_xor.as_u64(), 6);
    }
    /// Comparison Operators
    #[test]
    fn test_std_gt() {
        let gt0 = Value::try_from_init(7, 16).unwrap();
        let gt1 = Value::try_from_init(3, 16).unwrap();
        let std_gt = StdGt::new(16);
        let res_gt = std_gt.execute_bin(&gt0, &gt1);
        assert_eq!(res_gt.as_u64(), 1);
    }
    #[test]
    fn test_std_lt() {
        let lt0 = Value::try_from_init(7, 16).unwrap();
        let lt1 = Value::try_from_init(3, 16).unwrap();
        let std_lt = StdLt::new(16);
        let res_lt = std_lt.execute_bin(&lt0, &lt1);
        assert_eq!(res_lt.as_u64(), 0);
    }
    #[test]
    fn test_std_eq() {
        let eq0 = Value::try_from_init(4, 16).unwrap();
        let eq1 = Value::try_from_init(4, 16).unwrap();
        let std_eq = StdEq::new(16);
        let res_eq = std_eq.execute_bin(&eq0, &eq1);
        assert_eq!(res_eq.as_u64(), 1);
    }
    #[test]
    fn test_std_neq() {
        let neq0 = Value::try_from_init(4, 16).unwrap();
        let neq1 = Value::try_from_init(4, 16).unwrap();
        let std_neq = StdNeq::new(16);
        let res_neq = std_neq.execute_bin(&neq0, &neq1);
        assert!(res_neq.as_u64() == 0);
    }
    #[test]
    fn test_std_ge() {
        let ge0 = Value::try_from_init(35, 8).unwrap();
        let ge1 = Value::try_from_init(165, 8).unwrap();
        let std_ge = StdGe::new(8);
        let res_ge = std_ge.execute_bin(&ge0, &ge1);
        assert_eq!(res_ge.as_u64(), 0);
    }
    #[test]
    fn test_std_le() {
        let le0 = Value::try_from_init(8, 4).unwrap();
        let le1 = Value::try_from_init(8, 4).unwrap();
        let std_le = StdLe::new(4);
        let res_le = std_le.execute_bin(&le0, &le1);
        assert_eq!(res_le.as_u64(), 1);
    }
}

#[cfg(test)]
mod val_test {
    use crate::values::Value;
    #[test]
    fn basic_print_test() {
        let v1 = Value::try_from_init(12, 5).unwrap();
        println!("12 with bit width 5: {}", v1);
        assert_eq!(v1.as_u64(), 12);
    }
    #[test]
    fn basic_print_test2() {
        let v1 = Value::try_from_init(33, 6).unwrap();
        println!("33 with bit width 6: {}", v1);
        assert_eq!(v1.as_u64(), 33);
    }
    #[test]
    fn too_few_bits() {
        let v_16_4 = Value::try_from_init(16, 4).unwrap();
        println!("16 with bit width 4: {}", v_16_4);
        assert_eq!(v_16_4.as_u64(), 0);
        let v_31_4 = Value::try_from_init(31, 4).unwrap();
        println!("31 with bit width 4: {}", v_31_4);
        let v_15_4 = Value::try_from_init(15, 4).unwrap();
        println!("15 with bit width 4: {}", v_15_4);
        assert_eq!(v_31_4.as_u64(), v_15_4.as_u64());
    }
    #[test]
    fn clear() {
        let v_15_4 = Value::try_from_init(15, 4).unwrap();
        let v_15_4 = v_15_4.clear();
        println!("15 with bit width 4 AFTER clear: {}", v_15_4);
        assert_eq!(v_15_4.as_u64(), 0);
    }
    #[test]
    fn ext() {
        let v_15_4 = Value::try_from_init(15, 4).unwrap();
        assert_eq!(v_15_4.as_u64(), v_15_4.ext(8).as_u64());
    }

    //is there even a point of sext, if bit_vec can't take negative numbers? Or can it?
}
