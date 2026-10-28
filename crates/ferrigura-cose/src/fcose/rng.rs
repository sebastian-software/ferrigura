//! The generator that replaces `Math.random` for one layout run: `withSeededRandom` in
//! `packages/mermaid/src/diagrams/architecture/architectureSeed.ts` (lines 13-31).
//!
//! The state is a `u32` that starts at the seed (`seed >>> 0`); each draw adds the constant,
//! mixes the state with `Math.imul` and the shifts, and returns the result over 2^32. Every
//! operation is the same wrapping `u32` arithmetic as the source, so the draws are the same bits.
//! A seed of 0 is not reproducible in Mermaid (it means "do not replace `Math.random`"), so the
//! engine takes seeds from 1 on and the caller must not pass 0.

/// The mulberry32 generator of `withSeededRandom`.
#[derive(Clone, Debug)]
pub(crate) struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub(crate) fn new(seed: u32) -> Mulberry32 {
        Mulberry32 { state: seed }
    }

    /// The next draw, in `[0, 1)`: `Math.random()` under `withSeededRandom`.
    pub(crate) fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        // `t ^= t + Math.imul(t ^ (t >>> 7), t | 61)`: the sum wraps to 32 bits in `^=`.
        let mixed = (t ^ (t >> 7)).wrapping_mul(t | 61);
        t ^= t.wrapping_add(mixed);
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}
