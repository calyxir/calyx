use std::io::Write;

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
pub fn mask_n_bits(n: usize) -> u64 {
    if n < 64 {
        (1 << n) - 1
    } else {
        0xffff_ffff_ffff_ffff
    }
}
