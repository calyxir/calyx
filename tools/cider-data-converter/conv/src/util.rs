use crate::numrep as nr;
use std::io::Write;

pub fn msb(width: u32) -> u8 {
    let rem = width % 8;
    1u8 << (if rem != 0 { rem - 1 } else { 7 }) // shift to the right by between 0 and 7
}

/// attempt to sign extend the input data from in_width to out_width
// /// this function assumes that you are 'okay' with treating the contents of the Untypednum as a signed number.
pub fn sign_extend_untyped(
    i: &nr::BinRep,
    in_width: u32,
    out_width: u32,
) -> nr::BinRep {
    let msb_idx = in_width.saturating_sub(1);
    let should_extend = (i) & (1 << msb_idx);
    if should_extend == 0 {
        return *i;
    }
    // otherwise, need to extend
    let pad_bits = out_width - in_width;
    if pad_bits > 64 {
        panic!(
            "sign extension error on input {:#x}: from {} to {}",
            i, in_width, out_width
        )
    }
    let mask_bits = ((1 << in_width) - 1) << msb_idx;
    mask_bits | *i
}

// TODO: seek low-level performance in this later

/// pad [b], of length [len], to [NUM_BYTES]
#[inline]
pub fn pad_bytes<const NUM_BYTES: usize>(
    b: &[u8],
    len: usize,
) -> [u8; NUM_BYTES] {
    let mut e: [u8; NUM_BYTES] = [0; NUM_BYTES];
    if let Ok(nb) = (&mut e[..len]).write(b) {
        assert!(nb == len);
        e
    } else {
        panic!("aa")
    }
    // dest.write(b)
}

// TODO: below could probably be a macro

#[inline]
pub fn mask_n_bits(n: usize) -> nr::BinRep {
    if n < 64 {
        (1 << n) - 1
    } else {
        0xffff_ffff_ffff_ffff
    }
}
