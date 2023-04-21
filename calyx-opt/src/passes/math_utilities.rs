/// Returns the minimum bit width needed to represents n states with zero inclusive. Panics otherwise.
/// Note: To represent the number `n`, you need to represent `n+1` states.
///
/// For example,
/// `get_bit_width_from(3)` == 2 // 3 states requires a minimum of 2 bits.
/// `get_bit_width_from(4)` == 2 // 4 states can be represented with exactly 2 bits.
/// `get_bit_width_from(5)` == 3 // 5 states requires a minimum of 3 bits.
#[inline(always)]
pub fn get_bit_width_from(states: u64) -> u64 {
    if states == 0_u64 || states == 1_u64 {
        return states;
    }
    for index in 0u8..63 {
        let x = (63 - index) as u64;
        if states & (1u64 << x) != 0 {
            // If n is a power of two, return x. Otherwise, x + 1.
            return if (states & (states - 1)) == 0u64 {
                x
            } else {
                x + 1
            };
        }
    }
    panic!();
}

/// To run the get_bit_width_from tests:
/// ```bash
/// cd calyx/src/passes && cargo test math_utilities
/// ```
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_bit_width_from_zero() {
        assert_eq!(get_bit_width_from(0), 0);
    }

    #[test]
    fn get_bit_width_from_one() {
        assert_eq!(get_bit_width_from(1), 1);
    }

    #[test]
    fn get_bit_width_from_three() {
        assert_eq!(get_bit_width_from(3), 2);
    }

    #[test]
    fn get_bit_width_from_four() {
        assert_eq!(get_bit_width_from(4), 2);
    }

    #[test]
    fn get_bit_width_from_in_between() {
        assert_eq!(get_bit_width_from(9), 4);
        assert_eq!(get_bit_width_from(10), 4);
        assert_eq!(get_bit_width_from(11), 4);
        assert_eq!(get_bit_width_from(12), 4);
        assert_eq!(get_bit_width_from(13), 4);
        assert_eq!(get_bit_width_from(14), 4);
        assert_eq!(get_bit_width_from(15), 4);
    }

    #[test]
    fn get_bit_width_near_multiples_of_two() {
        let mut input: u64 = 2;
        let mut expected: u64 = 1;
        while input < (2 << 15) {
            // 2^n - 1 bits should be represented by n bits.
            assert_eq!(get_bit_width_from(input - 1), expected);
            // 2^n bits should be represented by n bits.
            assert_eq!(get_bit_width_from(input), expected);
            // 2^n + 1 bits should be represented by n + 1 bits.
            assert_eq!(get_bit_width_from(input + 1), expected + 1);

            input <<= 1;
            expected += 1;
        }
    }

    #[test]
    fn get_bit_width_from_large_numbers() {
        assert_eq!(get_bit_width_from(2u64.pow(61)), 61);
        assert_eq!(get_bit_width_from(2u64.pow(62)), 62);
        assert_eq!(get_bit_width_from(2u64.pow(63)), 63);
        assert_eq!(get_bit_width_from(18446744073709551614), 64); // 2^64 - 1
    }
}
