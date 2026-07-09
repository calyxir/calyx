use crate::numrep::*;
use crate::vbfp::*;

// implementation of additional features relating to number representation

// the below is to make the implementations in TypeSpec more manageable
// TODO: is it worth making specific, static variants of these?

pub fn float_read(
    s: String,
    _end: Endian,
    width: usize,
) -> Result<BinRep, ReadStringErr> {
    match width {
        32 => Ok(s.parse::<f32>()?.to_bits() as u64),
        64 => Ok(s.parse::<f64>()?.to_bits() as u64),
        _ => Err(ReadStringErr::from(format!(
            "undefined width {} for float",
            width
        ))),
    }
}

pub fn float_write(b: BinRep, _end: Endian, width: usize) -> String {
    match width {
        32 => f32::from_bits(b as u32).to_string(),
        64 => f64::from_bits(b as u64).to_string(),
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
) -> Result<BinRep, ReadStringErr> {
    if width <= 32 {
        let r = if signed {
            s.parse::<i32>()? as u64
        } else {
            s.parse::<u32>()? as u64
        };
        return Ok(r);
    } else if width <= 64 {
        let r = if signed {
            s.parse::<i64>()? as u64
        } else {
            s.parse::<u64>()? as u64
        };
        return Ok(r);
    } else {
        return Err(ReadStringErr::from(format!(
            "undefined width-signedness pair {}, {} for int. use bits instead?",
            width, signed
        )));
    }
}

// TODO: 'cheating' things out by extending to 64-bits might be problematic.
// this doesn't have the guarantee of masking out unnecessary bits which the 'as u32/i32' approach does
// TODO: maybe easier with tryfrom?

// not 'directly' bit lossless as before
pub fn int_write(
    b: BinRep,
    _end: Endian,
    width: usize,
    signed: bool,
) -> String {
    if signed {
        let msb_mask = 1 << (width - 1);
        let sgn_mask =
            !crate::util::mask_n_bits(width) & crate::util::mask_n_bits(64); // maximum width
        let extended = if b & msb_mask != 0 { sgn_mask | b } else { b };

        return format!("{}", extended as i64);
    } else {
        if width <= 64 {
            return format!("{}", b as u64);
        }
    }
    panic!("unknown width when writing out an int");
}

pub fn bits_read(s: String, _end: Endian) -> Result<BinRep, ReadStringErr> {
    let r = s.parse::<u64>()?;
    Ok(r)
}

pub fn bits_write(b: BinRep, _end: Endian) -> String {
    format!("{:#x}", b)
}

pub fn fixed_read(
    s: String,
    _end: Endian,
    width: usize,
    signed: bool,
    exp_mag: i32,
) -> Result<BinRep, ReadStringErr> {
    let tmp_b = float_read(s, _end, 64)?;
    let tmp_f = f64::from_bits(tmp_b);
    let fixed_equiv = FixedDef {
        total_size: width,
        exp_mag,
    };
    let r = fixed_equiv.from_fp_rounded(tmp_f, signed)?;
    Ok(r)
}

pub fn fixed_write(
    b: BinRep,
    _end: Endian,
    width: usize,
    signed: bool,
    exp_mag: i32,
) -> String {
    let fixed_equiv = FixedDef {
        total_size: width,
        exp_mag,
    };

    let r = fixed_equiv.to_fp_rounded(&b, signed);
    format!("{}", r)
}

// we can't always tell from bytes alone whether a number is 'correctly' typed, so instead just use same byteslice-based Thing for all of them
pub fn try_from_bytes(
    b: &[u8],
    len: usize,
    width: usize,
    end: Endian,
) -> Result<BinRep, CheckedConvErr> {
    let padded_bytes = crate::util::pad_bytes::<8>(b, len);
    let r = match end {
        Endian::Little => u64::from_le_bytes(padded_bytes),
        Endian::Big => u64::from_be_bytes(padded_bytes),
    };

    // mask to WIDTH bits
    Ok(r & crate::util::mask_n_bits(width))
}

#[cfg(test)]

mod tests {
    // use crate::numrep::ReprType;

    use super::*;
    // TODO: rewrite fixed-point benchmarks

    #[test]
    fn test_fixed_from_string() {
        let result =
            fixed_read(String::from("-0.5"), Endian::Little, 32, true, 16)
                .unwrap();
        let t: u32 = (result & 0xffff_ffff) as u32;

        // test by getting bits from 0.5 float directly using fixed
        let equiv = try_from_bytes(
            &FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_le_bytes(),
            4,
            32,
            Endian::Little,
        )
        .unwrap();
        assert_eq!((equiv & 0xffff_ffff) as u32, t)
    }

    #[test]
    fn test_fixed_roundtrip() {
        // attempt to roundtrip a value in bytes through the fixed expression
        let orig_bits =
            FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_bits();

        let thru_bits =
            try_from_bytes(&orig_bits.to_le_bytes(), 4, 32, Endian::Little)
                .unwrap();

        let out_bits = (thru_bits & 0xffff_ffff) as i32;

        assert_eq!(out_bits, orig_bits);
    }

    #[test]
    fn test_fixed_to_string() {
        let equiv = try_from_bytes(
            &FixedI32::<fixed::types::extra::U16>::from_num(-0.5).to_le_bytes(),
            4,
            32,
            Endian::Little,
        )
        .unwrap();
        assert_eq!(fixed_write(equiv, Endian::Little, 32, true, 16), "-0.5");
    }

    #[test]
    fn test_float_roundtrip() {
        let orig_bytes = (0.75_f64).to_le_bytes();

        let thru_bits =
            try_from_bytes(&orig_bytes, 8, 64, Endian::Little).unwrap();

        assert_eq!(thru_bits, u64::from_le_bytes(orig_bytes));
    }

    #[test]
    fn test_float_from_string() {
        use std::str::FromStr;
        let ref_bits = f32::from_str("0.123").unwrap().to_bits();

        let comp_bits =
            float_read(String::from("0.123"), Endian::Little, 32).unwrap();
        assert_eq!(ref_bits as u64, comp_bits);
    }

    #[test]
    fn test_float_to_string() {
        let orig_bytes = (0.752_f64).to_le_bytes();

        let equiv = try_from_bytes(&orig_bytes, 8, 64, Endian::Little).unwrap();
        assert_eq!(float_write(equiv, Endian::Little, 64), "0.752");
    }

    #[test]
    fn test_int_roundtrip() {
        let orig_bytes = (-1_i64).to_le_bytes();

        let thru_bits =
            try_from_bytes(&orig_bytes, 8, 64, Endian::Little).unwrap();

        assert_eq!(thru_bits, u64::from_le_bytes(orig_bytes));
    }

    #[test]
    fn test_int_from_string() {
        let ref_bits = u64::from_str_radix("ffffffff", 16).unwrap();
        let comp_bits =
            int_read(String::from("4294967295"), Endian::Little, 64, false)
                .unwrap();
        assert_eq!(ref_bits as u64, comp_bits);
    }

    #[test]
    fn test_int_to_string() {
        let orig_bytes = (-345_i64).to_le_bytes();
        let equiv = try_from_bytes(&orig_bytes, 8, 64, Endian::Little).unwrap();
        assert_eq!(int_write(equiv, Endian::Little, 32, true), "-345");
    }
}
