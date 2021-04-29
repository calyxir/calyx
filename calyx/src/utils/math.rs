fn bits_helper(n: u64, i: u64) -> u64 {
    if n == 0 {
        i
    } else {
        bits_helper(n / 2, i + 1)
    }
}

pub fn bits_needed_for(n: u64) -> u64 {
    bits_helper(n - 1, 0)
}
