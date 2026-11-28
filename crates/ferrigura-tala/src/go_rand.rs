// SPDX-License-Identifier: BSD-3-Clause
// Copyright 2009 The Go Authors. All rights reserved.
//
// Adapted from Go 1.27.4 src/math/rand/{rng,rand}.go. See LICENSE-GO.

mod seed_mix;

use seed_mix::SEED_MIX;

const STATE_LENGTH: usize = SEED_MIX.len();
const TAP_DISTANCE: usize = 273;
const SEED_MODULUS: i64 = (1 << 31) - 1;
const INT63_RANGE: u64 = 1 << 63;
const MAX_SMALL_BOUND: usize = i32::MAX as usize;

/// The deterministic Mitchell–Reeds stream used by Go's rand.NewSource.
pub(crate) struct GoRand {
    state: [u64; STATE_LENGTH],
    tap: usize,
    feed: usize,
}

impl GoRand {
    pub(crate) fn new(seed: i64) -> Self {
        let mut seed = seed.rem_euclid(SEED_MODULUS).unsigned_abs();
        if seed == 0 {
            seed = 89_482_311;
        }
        for _ in 0..20 {
            advance_seed(&mut seed);
        }

        let state = SEED_MIX.map(|mix| {
            let seed_word = advance_seed(&mut seed) << 40
                ^ advance_seed(&mut seed) << 20
                ^ advance_seed(&mut seed);
            seed_word ^ u64::from_ne_bytes(mix.to_ne_bytes())
        });
        Self {
            state,
            tap: 0,
            feed: STATE_LENGTH - TAP_DISTANCE,
        }
    }

    fn int63(&mut self) -> u64 {
        self.tap = previous_index(self.tap);
        self.feed = previous_index(self.feed);
        let next = self.state[self.feed].wrapping_add(self.state[self.tap]);
        self.state[self.feed] = next;
        next & (INT63_RANGE - 1)
    }

    fn uint32(&mut self) -> u32 {
        u32::try_from(self.int63() >> 31).expect("top 32 bits of a 63-bit value")
    }

    pub(crate) fn float64(&mut self) -> f64 {
        loop {
            // Converting to f64 first preserves Go's rounding. A value near
            // 2^63 can round up to 1.0, in which case Go takes another draw.
            #[allow(clippy::cast_precision_loss)]
            let fraction = self.int63() as f64 / INT63_RANGE as f64;
            if fraction < 1.0 {
                return fraction;
            }
        }
    }

    fn int63_below(&mut self, upper: u64) -> u64 {
        if upper.is_power_of_two() {
            return self.int63() & (upper - 1);
        }
        let max = INT63_RANGE - 1 - INT63_RANGE % upper;
        loop {
            let draw = self.int63();
            if draw <= max {
                return draw % upper;
            }
        }
    }

    fn uint32_below<E>(
        &mut self,
        upper: u32,
        step: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<u32, E> {
        let threshold = upper.wrapping_neg() % upper;
        loop {
            step()?;
            let product = u64::from(self.uint32()) * u64::from(upper);
            // Rejection depends on the low word, while the resulting index
            // is the high word of the product.
            #[allow(clippy::cast_possible_truncation)]
            if product as u32 >= threshold {
                return Ok(u32::try_from(product >> 32).expect("high word of the product"));
            }
        }
    }

    pub(crate) fn shuffle<T, E>(
        &mut self,
        values: &mut [T],
        mut step: impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        for end in (1..values.len()).rev() {
            let bound = end + 1;
            let selected = if bound <= MAX_SMALL_BOUND {
                let upper = u32::try_from(bound).expect("small shuffle bound");
                usize::try_from(self.uint32_below(upper, &mut step)?).expect("valid slice index")
            } else {
                // For the wide branch the caller charges a single step,
                // then the compatibility sampler owns any rejections.
                step()?;
                let upper = u64::try_from(bound).expect("shuffle bound fits in 64 bits");
                usize::try_from(self.int63_below(upper)).expect("valid slice index")
            };
            values.swap(end, selected);
        }
        Ok(())
    }
}

fn previous_index(index: usize) -> usize {
    if index == 0 {
        STATE_LENGTH - 1
    } else {
        index - 1
    }
}

fn advance_seed(seed: &mut u64) -> u64 {
    *seed = (48_271 * *seed) % SEED_MODULUS.unsigned_abs();
    *seed
}

#[cfg(test)]
mod tests;
