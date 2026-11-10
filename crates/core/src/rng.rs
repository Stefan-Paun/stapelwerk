//! Deterministic pseudo-random numbers.
//!
//! - [`SplitMix64`] drives tests, fuzzing (`fuzz-gen`) and the random callee-saved register values of
//!   function tests. Same seed, same sequence, on every platform.
//! - [`xorshift32`] is the generator behind the `RAND` system call (see `docs/ISA.md`).

/// SplitMix64 (Steele, Lea, Flood 2014). Small, fast, and good enough for tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Creates a generator from a seed. Every seed (including 0) is valid.
    pub const fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next 32 random bits (the high half of [`SplitMix64::next_u64`]).
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform-enough value in `0..n` (multiply-shift; negligible bias). Returns 0 for `n == 0`.
    pub fn below(&mut self, n: u64) -> u64 {
        ((self.next_u64() as u128 * n as u128) >> 64) as u64
    }

    /// Value in the inclusive range `lo..=hi`. Returns `lo` if `hi < lo`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi as i128 - lo as i128 + 1) as u128;
        if span > u64::MAX as u128 {
            return self.next_u64() as i64;
        }
        lo.wrapping_add(self.below(span as u64) as i64)
    }

    /// `true` with probability `num / den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }
}

/// Start value of the `RAND` generator when the seed is 0 (xorshift32 would stay 0 forever).
pub const RAND_ZERO_SEED: u32 = 0x9E37_79B9;

/// Initial `RAND` state for a run seed.
pub const fn rand_state_from_seed(seed: u32) -> u32 {
    if seed == 0 { RAND_ZERO_SEED } else { seed }
}

/// One step of Marsaglia's xorshift32 with shifts (13, 17, 5). `RAND` stores the returned value as
/// the new state and also returns it in r0.
pub const fn xorshift32(state: u32) -> u32 {
    let mut x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_reference_values() {
        // Reference sequence for seed 0 (from the published C implementation).
        let mut r = SplitMix64::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(r.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn ranges_stay_inside() {
        let mut r = SplitMix64::new(7);
        for _ in 0..10_000 {
            let v = r.range_i64(-3, 3);
            assert!((-3..=3).contains(&v));
            assert!(r.below(10) < 10);
        }
        assert_eq!(r.below(0), 0);
        assert_eq!(r.range_i64(5, 5), 5);
    }

    #[test]
    fn xorshift_reference() {
        // xorshift32(1) with (13, 17, 5): 1 ^ 1<<13 = 0x2001; ^ >>17 = 0x2001; ^ <<5 = 0x42021
        assert_eq!(xorshift32(1), 0x0004_2021);
        assert_eq!(rand_state_from_seed(0), RAND_ZERO_SEED);
        assert_ne!(xorshift32(RAND_ZERO_SEED), 0);
    }
}
