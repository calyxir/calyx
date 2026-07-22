use baa::{BitVecOps, BitVecValue};

use crate::typing::{Endian, NumParseErr};
use crate::vbfp::*;

// implementation of additional features relating to number representation

// the below read/write implementations are to make the implementations in TypeSpec more manageable
// TODO: is it worth making specific, static variants of these?

#[inline]
pub fn is_hexstring(s: &str) -> bool {
    s.starts_with("0x")
}

pub fn read_hexstring(
    s: &str,
    _end: Endian,
    width: usize,
) -> Result<baa::BitVecValue, NumParseErr> {
    let Some(cleaned_str) = s.trim_start().strip_prefix("0x") else {
        return Err(NumParseErr::HexRead(format!(
            "could not strip prefix from {}",
            s
        )));
    };
    let Ok(val) = baa::BitVecValue::from_hex_str(cleaned_str) else {
        return Err(NumParseErr::HexRead(format!(
            "baa could not read {} as hex",
            s
        )));
    };

    if val.width() > (width as u32) {
        return Err(NumParseErr::HexRead(format!(
            "bad hexstring {}: incorrect width, not {}",
            s, width
        )));
    }
    Ok(val)
}

pub fn float_read(
    s: String,
    _end: Endian,
    width: usize,
) -> Result<baa::BitVecValue, NumParseErr> {
    match width {
        32 => Ok(BitVecValue::from_bytes_le(
            &s.parse::<f32>()?.to_le_bytes(),
            32,
        )),
        64 => Ok(BitVecValue::from_bytes_le(
            &s.parse::<f64>()?.to_le_bytes(),
            64,
        )),
        _ => Err(NumParseErr::Width(width, crate::typing::TypeClass::Float)),
    }
}

pub fn float_write(b: &baa::BitVecValue, _end: Endian, width: usize) -> String {
    debug_assert!(b.width() == 32 || b.width() == 64);
    let bits = b.to_u64().unwrap();
    match width {
        32 => f32::from_bits(bits as u32).to_string(),
        64 => f64::from_bits(bits).to_string(),
        _ => panic!("unknown width when writing out a float"),
    }
}

// TODO: this conditional-heavy thing can probably be simplified
// TODO: probably needs range checks
pub fn int_read(
    s: String,
    _end: Endian,
    width: usize,
    signed: bool,
) -> Result<BitVecValue, NumParseErr> {
    if !signed && s.contains('-') {
        return Err(NumParseErr::Misc(
            "found sign when parsing unsigned int".to_string(),
        ));
    }
    BitVecValue::from_str_radix(&s, 10, width as u32)
        .map_err(|e| NumParseErr::Baa(s, e))
}

pub fn int_write(
    b: &BitVecValue,
    _end: Endian,
    _width: usize,
    signed: bool,
) -> String {
    if signed && b.is_negative() {
        let tmp = &b.negate();
        format!("-{}", tmp.to_dec_str())
    } else {
        b.to_dec_str()
    }
}

pub fn bits_read(
    s: String,
    _end: Endian,
    width: usize,
) -> Result<BitVecValue, NumParseErr> {
    BitVecValue::from_str_radix(&s, 2, width as u32)
        .map_err(|e| NumParseErr::Baa(s, e))
}

#[inline]
pub fn bits_write(b: &BitVecValue, _end: Endian) -> String {
    format!("0b{}", b.to_bit_str())
}

pub fn fixed_read(
    s: String,
    _end: Endian,
    width: usize,
    signed: bool,
    exp_mag: i32,
) -> Result<BitVecValue, NumParseErr> {
    let tmp_b = float_read(s, _end, 64)?;
    let Some(bits_trunc) = tmp_b.to_u64() else {
        return Err(NumParseErr::Width(
            tmp_b.width() as usize,
            crate::typing::TypeClass::Fixed { exp_mag },
        ));
    };
    let tmp_f = f64::from_bits(bits_trunc);
    let fixed_equiv = FixedDef {
        total_size: width,
        exp_mag,
        signed,
    };
    let r = fixed_equiv.from_fp_rounded(tmp_f)?;
    Ok(r)
}

pub fn fixed_write(
    b: &BitVecValue,
    _end: Endian,
    width: usize,
    signed: bool,
    exp_mag: i32,
) -> String {
    let fixed_equiv = FixedDef {
        total_size: width,
        exp_mag,
        signed,
    };

    let r = fixed_equiv.to_fp_rounded(b);
    format!("{}", r)
}

// we can't always tell from bytes alone whether a number is 'correctly' typed, so instead just use same byteslice-based Thing for all of them
pub fn try_from_bytes(
    b: &[u8],
    width: usize,
    end: Endian,
) -> Result<BitVecValue, NumParseErr> {
    let r = match end {
        Endian::Little => BitVecValue::from_bytes_le(b, width as u32),
        Endian::Big => unimplemented!(),
    };

    debug_assert!(r.width() == width as u32);

    Ok(r)
}

#[cfg(test)]
mod tests {

    // use crate::numrep::ReprType;

    use super::*;

    // #[test]
    // fn get_fixed_refs() {
    //     let e = fixed::FixedI32::<fixed::types::extra::U16>::from_str("-0.5")
    //         .unwrap();
    //     println!("{:#x}", e.to_bits())
    // }

    #[test]
    fn test_fixed_from_string() {
        let result =
            fixed_read(String::from("-0.5"), Endian::Little, 32, true, 16)
                .unwrap();

        // test by getting bits from -0.5 float directly using fixed
        let equiv_bits = 0xffff_8000_u32;
        let equiv =
            try_from_bytes(&equiv_bits.to_le_bytes(), 32, Endian::Little)
                .unwrap();
        assert_eq!(equiv, result)
    }

    #[test]
    fn test_fixed_roundtrip() {
        // attempt to roundtrip a value in bytes through the fixed expression
        let orig_bits = 0x0000_8000_u32; // +0.5

        let thru_bits =
            try_from_bytes(&orig_bits.to_le_bytes(), 32, Endian::Little)
                .unwrap();

        let out_bits = thru_bits.to_u64().unwrap();

        assert_eq!(orig_bits as u64, out_bits);
    }

    #[test]
    fn test_fixed_to_string() {
        let orig_bits = 0xffff_8000_u32; // -0.5

        let equiv =
            try_from_bytes(&orig_bits.to_le_bytes(), 32, Endian::Little)
                .unwrap();
        assert_eq!(fixed_write(&equiv, Endian::Little, 32, true, 16), "-0.5");
    }

    #[test]
    fn test_float_roundtrip() {
        let orig_bytes = (0.75_f64).to_le_bytes();

        let thru_bits =
            try_from_bytes(&orig_bytes, 64, Endian::Little).unwrap();

        assert_eq!(thru_bits.to_u64().unwrap(), u64::from_le_bytes(orig_bytes));
    }

    #[test]
    fn test_float_from_string() {
        use std::str::FromStr;
        let ref_bits = f32::from_str("0.123").unwrap().to_bits();

        let comp_bits =
            float_read(String::from("0.123"), Endian::Little, 32).unwrap();
        assert_eq!(ref_bits as u64, comp_bits.to_u64().unwrap());
    }

    #[test]
    fn test_float_to_string() {
        let orig_bytes = (0.752_f64).to_le_bytes();

        let equiv = try_from_bytes(&orig_bytes, 64, Endian::Little).unwrap();
        assert_eq!(float_write(&equiv, Endian::Little, 64), "0.752");
    }

    #[test]
    fn test_int_roundtrip() {
        let orig_bytes = (-1_i64).to_le_bytes();

        let thru_bits =
            try_from_bytes(&orig_bytes, 64, Endian::Little).unwrap();

        assert_eq!(thru_bits.to_u64().unwrap(), u64::from_le_bytes(orig_bytes));
    }

    #[test]
    fn test_int_from_string() {
        let ref_bits = u64::from_str_radix("ffffffff", 16).unwrap();
        let comp_bits =
            int_read(String::from("4294967295"), Endian::Little, 64, false)
                .unwrap();
        assert_eq!(ref_bits, comp_bits.to_u64().unwrap());

        let ref_bits = u64::MAX;
        let comp_bits =
            int_read(String::from("-1"), Endian::Little, 64, true).unwrap();
        assert_eq!(ref_bits, comp_bits.to_u64().unwrap());
    }

    #[test]
    fn test_int_to_string() {
        let orig_bytes = (-345_i64).to_le_bytes();
        let equiv = try_from_bytes(&orig_bytes, 64, Endian::Little).unwrap();
        assert_eq!(int_write(&equiv, Endian::Little, 32, true), "-345");
    }

    #[test]
    fn test_bits_from_string() {
        let ref_bits = u64::from_str_radix("4", 16).unwrap();
        let comp_bits =
            bits_read(String::from("100"), Endian::Little, 64).unwrap();
        assert_eq!(ref_bits, comp_bits.to_u64().unwrap());
    }
}
