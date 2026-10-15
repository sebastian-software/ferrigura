//! `java.util.Random` as GWT emulates it in elkjs 0.9.3.
//!
//! GWT keeps the 48-bit seed in two 24-bit halves and computes the same linear
//! congruential generator as the JVM. Two deviations matter here:
//!
//! - `nextInt(bound)` for a bound that is not a power of two never retries.
//!   The JVM retries when `bits - val + (bound - 1)` overflows `int`; the
//!   JavaScript emulation computes that sum in doubles, so the test never fires.
//! - `nextInt(bound)` for a power of two uses `round_int`, which truncates, so
//!   it equals the JVM result.
//!
//! Source: `java/util/Random` in GWT, `lib/elk-worker.js` of elkjs 0.9.3
//! (`Random_0`, `$nextInternal`, `$nextInt`, `$nextDouble`).

use crate::java::math::round_int;

const MULTIPLIER: u64 = 0x5_DEEC_E66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1 << 48) - 1;

/// The generator behind `new Random(seed)`.
#[derive(Debug, Clone)]
pub struct Random {
    seed: u64,
}

impl Random {
    /// `new Random(seed)`: the seed is scrambled with the multiplier, as the JVM does.
    pub fn new(seed: i64) -> Self {
        Random {
            seed: ((seed as u64) ^ MULTIPLIER) & MASK,
        }
    }

    /// `next(bits)` of the JVM: the top `bits` bits of the next 48-bit state, as an `int`.
    fn next_internal(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        // `bits` is 1..=32, so the shift stays in range; the cast wraps like Java's.
        (self.seed >> (48 - bits)) as i32
    }

    /// `nextInt()`.
    pub fn next_int(&mut self) -> i32 {
        self.next_internal(32)
    }

    /// `nextInt(bound)` for `bound > 0`, as elkjs computes it.
    ///
    /// # Panics
    ///
    /// Panics when `bound` is not positive, as `checkCriticalArgument` does.
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        if bound & bound.wrapping_neg() == bound {
            // Power of two: the JVM's `(bound * (long) next(31)) >> 31`.
            let r = self.next_internal(31) as f64;
            // Division by 2^31 is exact, as is the multiplication by GWT's 2^-31 literal.
            return round_int(f64::from(bound) * r / 2_147_483_648.0);
        }
        let bits = self.next_internal(31);
        bits % bound
    }

    /// `nextDouble()` in `[0, 1)`.
    pub fn next_double(&mut self) -> f64 {
        f64::from(self.next_internal(26)) * 1.490_116_119_384_765_6E-8
            + f64::from(self.next_internal(27)) * 1.110_223_024_625_156_5E-16
    }

    /// `nextBoolean()`.
    pub fn next_boolean(&mut self) -> bool {
        self.next_internal(1) != 0
    }

    /// `nextFloat()` in `[0, 1)`: 24 bits over 2^24, which a float holds exactly.
    pub fn next_float(&mut self) -> f64 {
        f64::from(self.next_internal(24)) / 16_777_216.0
    }

    /// `nextLong()`: two `next(32)` steps, the first one the high half, both signed.
    pub fn next_long(&mut self) -> i64 {
        (i64::from(self.next_internal(32)) << 32).wrapping_add(i64::from(self.next_internal(32)))
    }

    /// `setSeed(seed)`: restarts the generator as `new Random(seed)` does.
    pub fn set_seed(&mut self, seed: i64) {
        *self = Random::new(seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_int_matches_the_jvm_for_known_seeds() {
        // Values published for java.util.Random; GWT produces the same stream.
        assert_eq!(Random::new(42).next_int(), -1_170_105_035);
        assert_eq!(Random::new(0).next_int(), -1_155_484_576);
    }

    #[test]
    fn matches_elkjs_bound_sequences() {
        // Reference values produced by elkjs 0.9.3's GWT Random (`$nextInt`, `$nextDouble`).
        let mut random = Random::new(42);
        let values: Vec<i32> = (0..5).map(|_| random.next_int_bounded(10)).collect();
        assert_eq!(values, vec![0, 3, 8, 4, 0]);

        let mut random = Random::new(1);
        let values: Vec<i32> = (0..8).map(|_| random.next_int_bounded(7)).collect();
        assert_eq!(values, vec![4, 4, 1, 0, 6, 6, 0, 1]);

        let mut random = Random::new(1);
        let values: Vec<i32> = (0..4).map(|_| random.next_int_bounded(16)).collect();
        assert_eq!(values, vec![11, 1, 6, 6]);

        assert_eq!(Random::new(-5).next_int(), 1_156_638_822);
    }

    #[test]
    fn next_double_matches_elkjs() {
        let mut random = Random::new(1);
        let values: Vec<f64> = (0..3).map(|_| random.next_double()).collect();
        assert_eq!(values[0], 0.730_878_190_703_290_9);
        assert_eq!(values[1], 0.410_080_811_492_201_66);
        assert_eq!(values[2], 0.207_714_841_309_717_07);
    }

    #[test]
    fn bounded_values_stay_in_range() {
        let mut random = Random::new(1);
        for _ in 0..1000 {
            let value = random.next_int_bounded(7);
            assert!((0..7).contains(&value));
        }
    }

    #[test]
    fn doubles_are_in_unit_interval() {
        let mut random = Random::new(1);
        for _ in 0..1000 {
            let value = random.next_double();
            assert!((0.0..1.0).contains(&value));
        }
    }
}
