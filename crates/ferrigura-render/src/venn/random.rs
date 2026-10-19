//! The random numbers of the venn layout. The reference draws them from `Math.random` when the
//! layout has eight areas or more (`docs-port/venn/NOTES.md` L4): the geometry harness replaces
//! `Math.random` with a generator seeded by the clock of the render (`tools/port/geometry/common.ts`,
//! `renderReference`). The port takes the same generator, so that its draws are the reference's
//! under the harness, in the same order.

/// The generator of the harness, seeded with `seed` (`clock.seed >>> 0`): the same arithmetic as
/// `renderReference`, in 32-bit integers. Each call returns the next number in `[0, 1)`.
pub(super) fn seeded(seed: u32) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state.wrapping_add(0x6d2b_79f5);
        let mut t = (state ^ (state >> 15)).wrapping_mul(1 | state);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::seeded;

    /// The first four draws of seed 1, as the JavaScript of the harness gives them (computed by
    /// Node 24 with the source of `renderReference`, `tools/port/geometry/common.ts`).
    #[test]
    fn the_draws_are_the_harness_generators_for_seed_one() {
        let mut random = seeded(1);
        let draws: Vec<f64> = (0..4).map(|_| random()).collect();
        assert_eq!(
            draws,
            [
                0.627_073_940_588_161_3,
                0.002_735_721_180_215_478,
                0.527_447_039_959_952_2,
                0.981_050_967_471_674_1,
            ]
        );
    }
}
