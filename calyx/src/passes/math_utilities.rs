/// Returns the ceiling log2 of an integer n. Panics if not .
///
/// Currently, Rust does not support logarithmic functions on
/// integral types. For the most recent discussion, see:
/// https://github.com/rust-lang/rust/issues/70887
pub fn log2_ceil(n: u64) -> u64 {
    for index in 0u8..63 {
        if n & (1u64 << (63 - index)) != 0 {
            return (63 - index) as u64;
        }
    }
    panic!();
}
