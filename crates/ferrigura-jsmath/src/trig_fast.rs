//! A fast path for `sin` and `cos`: double-double evaluation with a proven error bound, and a rounding
//! test that returns a result only when the whole error interval rounds to one double. When the test
//! is not conclusive, the caller takes the fixed-point evaluator in `correct.rs`, so the result is the
//! correctly rounded value either way. Written for this crate.
//!
//! Every operation is an ordinary rounded double operation (`u = 2^-53`, round to nearest) or an
//! error-free transformation built from them: `two_sum` (Knuth) and `two_prod` (Dekker, with
//! Veltkamp's split). Nothing uses a fused multiply-add, so the result does not depend on the target.
//!
//! The argument `ax` is positive, at least `2^-100` (below that `correct.rs` answers itself) and at
//! most [`LIMIT`]; larger arguments go to the fixed-point path. The steps and the bound of each:
//!
//! 1. [`reduce`]: `r = ax - k pi/2` as a double-double `rh + rl`, `|r| <= 0.7855`, with a computed
//!    absolute error bound `err`.
//! 2. [`series`]: `S(z)` or `C(z)` at `z = r^2`, where `sin r = r + r z S(z)` and
//!    `cos r = 1 + z C(z)`, with absolute errors below `2^-93` and `2^-96`.
//! 3. [`sin_reduced`], [`cos_reduced`]: the value as `fh + fl`, an error bound `eps` for it, and
//!    [`round_test`].

/// The largest argument of the fast path. `k = round(ax 2/pi)` stays below `2^30`, the products with
/// the parts of `pi/2` stay exact, and `|ax 2/pi - k| <= 0.5 + 2^-22` (the error of the product
/// `ax * FRAC_2_PI` is at most `ax (2/pi) 2.0001u <= 1.6e-7`), so `|r| <= (pi/2)(0.5 + 2^-22) < 0.7854`.
const LIMIT: f64 = 1_073_741_824.0; // 2^30

/// `pi/2 = C1 + C2 + C3 + delta`, each part the double nearest to what is left, `|delta| < 2^-163`
/// (checked against the 256-bit constant in the tests of `correct.rs`).
pub(crate) const PIO2: [f64; 3] = [
    f64::from_bits(0x3ff921fb54442d18),
    f64::from_bits(0x3c91a62633145c07),
    f64::from_bits(0xb91f1976b7ed8fbc),
];

/// `(-1)^n / (2n+1)!` for `n = 1..=6` as double-doubles (`hi` the nearest double, `lo` the double
/// nearest to the rest, so `|c - hi - lo| <= u^2 |c|`), and for `n = 7..=13` as doubles. Checked
/// against fixed-point reciprocals of the factorials in the tests of `correct.rs`.
pub(crate) const SIN_DD: [(f64, f64); 6] = [
    (
        f64::from_bits(0xbfc5555555555555),
        f64::from_bits(0xbc65555555555555),
    ),
    (
        f64::from_bits(0x3f81111111111111),
        f64::from_bits(0x3c01111111111111),
    ),
    (
        f64::from_bits(0xbf2a01a01a01a01a),
        f64::from_bits(0xbb6a01a01a01a01a),
    ),
    (
        f64::from_bits(0x3ec71de3a556c734),
        f64::from_bits(0xbb6c154f8ddc6c00),
    ),
    (
        f64::from_bits(0xbe5ae64567f544e4),
        f64::from_bits(0x3afc062e06d1f209),
    ),
    (
        f64::from_bits(0x3de6124613a86d09),
        f64::from_bits(0x3a8f28e0cc748ebe),
    ),
];
pub(crate) const SIN_TAIL: [f64; 7] = [
    f64::from_bits(0xbd6ae7f3e733b81f),
    f64::from_bits(0x3ce952c77030ad4a),
    f64::from_bits(0xbc62f49b46814157),
    f64::from_bits(0x3bd71b8ef6dcf572),
    f64::from_bits(0xbb4761b41316381a),
    f64::from_bits(0x3ab3f3ccdd165fa9),
    f64::from_bits(0xba1d1ab1c2dccea3),
];

/// `(-1)^n / (2n)!` for `n = 1..=7` as double-doubles and for `n = 8..=14` as doubles, as above.
pub(crate) const COS_DD: [(f64, f64); 7] = [
    (f64::from_bits(0xbfe0000000000000), 0.0),
    (
        f64::from_bits(0x3fa5555555555555),
        f64::from_bits(0x3c45555555555555),
    ),
    (
        f64::from_bits(0xbf56c16c16c16c17),
        f64::from_bits(0x3bef49f49f49f49f),
    ),
    (
        f64::from_bits(0x3efa01a01a01a01a),
        f64::from_bits(0x3b3a01a01a01a01a),
    ),
    (
        f64::from_bits(0xbe927e4fb7789f5c),
        f64::from_bits(0xbb3cbbc05b4fa99a),
    ),
    (
        f64::from_bits(0x3e21eed8eff8d898),
        f64::from_bits(0xbac2aec959e14c06),
    ),
    (
        f64::from_bits(0xbda93974a8c07c9d),
        f64::from_bits(0xba305d6f8a2efd1f),
    ),
];
pub(crate) const COS_TAIL: [f64; 7] = [
    f64::from_bits(0x3d2ae7f3e733b81f),
    f64::from_bits(0xbca6827863b97d97),
    f64::from_bits(0x3c1e542ba4020225),
    f64::from_bits(0xbb90ce396db7f853),
    f64::from_bits(0x3aff2cf01972f578),
    f64::from_bits(0xba688e85fc6a4e5a),
    f64::from_bits(0x39d0a18a2635085d),
];

/// `2^-50`, `2^-92`, `2^-95`, `2^-160`: the factors of the error bounds below.
const TWO_M50: f64 = f64::from_bits(0x3cd0000000000000);
const TWO_M92: f64 = f64::from_bits(0x3a30000000000000);
const TWO_M95: f64 = f64::from_bits(0x3a00000000000000);
const TWO_M160: f64 = f64::from_bits(0x35f0000000000000);

/// `s + e = a + b` exactly (Knuth's TwoSum, no condition on the operands).
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// `hi + lo = a` exactly, each half with at most 26 significant bits (Veltkamp's split, exact for
/// `|a| < 2^996`).
fn split(a: f64) -> (f64, f64) {
    let gamma = 134_217_729.0 * a; // 2^27 + 1
    let hi = gamma - (gamma - a);
    (hi, a - hi)
}

/// `p + e = a * b` exactly (Dekker's product), when nothing overflows and `e` is not subnormal. The
/// operands here are below `2^31` and the products above `2^-420`, far from both limits.
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    (p, ((ah * bh - p) + ah * bl + al * bh) + al * bl)
}

/// The double-double product `(ah + al)(bh + bl)`, not renormalized.
///
/// With `|al| <= x u |ah|` and `|bl| <= y u |bh|`, the result `ph + pl` differs from the exact product by
/// at most `(xy + 3x + 3y + 1) u^2 |ah bh|`: the dropped `al bl` (`xy`), the roundings of `ah bl`
/// (`y`), `al bh` (`x`), their sum (`x + y`), and the last sum, whose operands are below
/// `(1 + x + y) u |ah bh|` (`1 + x + y`). Terms of order `u^3` are covered by rounding the constants
/// up. And `|pl| <= (1 + x + y) u |ah bh| (1 + 3u)`.
fn mul_dd((ah, al): (f64, f64), (bh, bl): (f64, f64)) -> (f64, f64) {
    let (ph, pt) = two_prod(ah, bh);
    (ph, pt + (ah * bl + al * bh))
}

/// One double-double Horner step `c + z a`, renormalized.
///
/// The inputs: the coefficient `c = ch + cl` with `|cl| <= u |ch|`; `z = zh + zl` with
/// `zh <= 0.6171`, `|zl| <= 3.02 u zh` (from `Reduced::square`); `a = ah + al` with `|al| <= u |ah|`,
/// and `|c|, |a| <= 0.5001`. The result `rh + rl`, `|rl| <= u |rh|` (from `two_sum`), differs from
/// `(ch + cl) + z a` by at most, in units of `u^2`, with `A = |ah|` and `C = |ch|`:
///
/// - the dropped `zl al`: `3.02 * 0.6171 A = 1.864 A`;
/// - rounding `zh al` and `zl ah`: `0.618 A` and `1.864 A`; their sum `m`, below `2.483 u A`: `2.483 A`;
/// - `cl + pl`, below `u (C + 0.6171 A)(1 + u)`: `1.0001 (C + 0.6171 A)`;
/// - adding `se`, below `u (C + 0.6171 A)(1 + u)^2`: `2.0003 (C + 0.6171 A)`;
/// - adding `m`: `2.0004 C + 1.2345 A + 2.4833 A`;
/// - the representation error of the coefficient, `|c - ch - cl| <= u^2 C`: `C`.
///
/// That is at most `12.4 A + 6.01 C <= 9.21`, and `9.21 u^2 < 2^-102.7 < 2^-100` per step.
fn horner_step((ch, cl): (f64, f64), (zh, zl): (f64, f64), (ah, al): (f64, f64)) -> (f64, f64) {
    let (ph, pl) = two_prod(zh, ah);
    let m = zh * al + zl * ah;
    let (sh, se) = two_sum(ch, ph);
    two_sum(sh, ((cl + pl) + se) + m)
}

/// `sum c_n z^(n-1)` over the double-double coefficients `dd` followed by the double ones `tail`.
///
/// The tail is a degree-six polynomial evaluated by Horner's rule in doubles at `zh`: with the
/// rounding of its coefficients, its error is at most `gamma_13 sum |c_i| z^i` (Higham, *Accuracy and
/// Stability of Numerical Algorithms*, (5.3)), `gamma_13 <= 13.0001 u`, and the sum is below
/// `1.003 |c_first|`. Taking `zh` for `z` adds at most `|zl| |T'| <= 3.02 u * 0.6171 * 1.01 |c_second|`.
/// The double-double steps each add below `2^-100` ([`horner_step`]) and multiply the error that
/// comes in by `|z| <= 0.6171`, so the steps add at most `2^-100 / (1 - 0.6171) < 2^-98.6`.
///
/// For `S` (six steps, tail from `1/15!`): the tail gives `13.0001 u * 1.003 / 15! * 0.6171^6 =
/// 6.12e-29`, the truncation after `1/27!` is below the first term left out, `0.6171^13 / 29! <
/// 2.2e-34`, so the error is below `6.12e-29 + 2^-98.6 < 6.4e-29 < 2^-93`.
///
/// For `C` (seven steps, tail from `1/16!`): `13.0001 u * 1.003 / 16! * 0.6171^7 = 2.37e-30`, the
/// truncation after `1/28!` is below `0.6171^14 / 30! < 4.5e-36`, so the error is below
/// `2.37e-30 + 2^-98.6 < 4.5e-30 < 2^-96`.
fn series(z: (f64, f64), dd: &[(f64, f64)], tail: &[f64]) -> (f64, f64) {
    let mut t = 0.0;
    for &c in tail.iter().rev() {
        t = c + z.0 * t;
    }
    let mut a = (t, 0.0);
    for &c in dd.iter().rev() {
        a = horner_step(c, z, a);
    }
    a
}

/// `RN(v)` for every `v` within `eps` of `hi + lo`, or `None` when they do not all round alike.
///
/// Let `E` be a true bound and `eps (1 - u) >= E + u |lo|`. Then `RN(lo - eps) <= lo - E`, so by the
/// monotonicity of rounding the lower result is at most `RN(hi + lo - E)`, and likewise the upper
/// result is at least `RN(hi + lo + E)`; when both agree, so does every value in between. The bounds
/// `eps` below are at least three times the error `E` they cover, which pays for `u |lo|`, the
/// factors `1 - u` and the roundings in computing `eps`.
fn round_test([hi, lo, eps]: [f64; 3]) -> Option<f64> {
    let lower = hi + (lo - eps);
    let upper = hi + (lo + eps);
    (lower == upper).then_some(lower)
}

/// `ax` as `k pi/2 + r`: the quadrant `k mod 4`, `r = rh + rl` with `|rl| <= u |rh|`, and a bound
/// `err` on `|ax - k pi/2 - r|`.
struct Reduced {
    quadrant: u32,
    rh: f64,
    rl: f64,
    err: f64,
}

/// Cody and Waite's reduction with the three parts of `pi/2`, for `0 < ax <= LIMIT`.
///
/// `k C1 = ph + pl` and `k C2 = qh + ql` exactly (`two_prod`), and `s + e = ax - ph`, `h1 + l1 = s - pl`,
/// `h2 + l2 = h1 - qh` exactly (`two_sum`), so
///
/// `ax - k pi/2 = h2 + (e + l1 + l2 - ql - k C3) - k delta`.
///
/// The bracket is summed in doubles as `lo`, with `c3 = RN(k C3)`. Summing five terms costs at most
/// `gamma_4 <= 4.0001 u` times the sum of their magnitudes (Higham, (4.4)), and `c3` adds `u |c3|`;
/// both are below `5.0001 u = 2^-50.68` times that sum, and `k |delta| < k 2^-163`. The computed
/// `err = 2^-50 sum + k 2^-160` exceeds that by more than half, which covers the roundings in
/// computing it. `rh + rl = h2 + lo` exactly (`two_sum`). For `k = 0` every term is zero and `r = ax`.
fn reduce(ax: f64) -> Option<Reduced> {
    if ax > LIMIT || ax.is_nan() {
        return None;
    }
    let k = (ax * std::f64::consts::FRAC_2_PI).round_ties_even();
    let (ph, pl) = two_prod(k, PIO2[0]);
    let (s, e) = two_sum(ax, -ph);
    let (qh, ql) = two_prod(k, PIO2[1]);
    let (h1, l1) = two_sum(s, -pl);
    let (h2, l2) = two_sum(h1, -qh);
    let c3 = k * PIO2[2];
    let lo = (((e + l1) + l2) - ql) - c3;
    let (rh, rl) = two_sum(h2, lo);
    let magnitudes = (((e.abs() + l1.abs()) + l2.abs()) + ql.abs()) + c3.abs();
    Some(Reduced {
        quadrant: ((k as u64) & 3) as u32,
        rh,
        rl,
        err: TWO_M50 * magnitudes + k * TWO_M160,
    })
}

impl Reduced {
    /// `z = r^2 = zh + zl`. `rh^2 = zh + t` exactly, and `zl = t + 2 RN(rh rl)`: the dropped `rl^2`
    /// (`u^2 rh^2`), the rounding of `rh rl` (`2u^2 rh^2`, doubled) and of the sum (below
    /// `3.01u^2 rh^2`) put `zh + zl` within `6.01 u^2 rh^2` of `r^2`, and `|zl| <= 3.02 u zh`.
    /// Since `|r| <= 0.7855`, `zh <= 0.6171`.
    fn square(&self) -> (f64, f64) {
        let (zh, t) = two_prod(self.rh, self.rh);
        (zh, t + 2.0 * (self.rh * self.rl))
    }
}

/// `sin r` as `fh + fl`, with the bound `eps` for [`round_test`].
///
/// `sin r = r (1 + w)` with `w = z S(z)`, `-0.1029 <= w <= 0`. The errors, relative to `|rh|`:
///
/// - `S` within `2^-93` ([`series`]), times `|r z| <= 0.6171 |rh| (1 + u)`: `2^-93.69`;
/// - `z` within `6.01 u^2 rh^2` of `r^2`, times `|d(zS)/dz| <= 1/6`, times `|r|`: below `2^-106`;
/// - `w = z S` by [`mul_dd`] with `x = 3.02`, `y = 1`: `16.1 u^2 |zh sh| <= 2.7 u^2 zh`, times `|r|`:
///   below `2^-105`, and `|wl| <= 5.05 u |wh|`;
/// - `r w` by [`mul_dd`] with `x = 1`, `y = 5.05`: `24.3 u^2 |rh wh| <= 2.5 u^2 |rh|`, and
///   `|ql| <= 0.73 u |rh|`;
/// - the last sums: `rl + ql` below `1.73 u |rh|` and `e + (rl + ql)` below `2.74 u |rh|`
///   (`|fh| <= |rh|`, as `w <= 0`), so `4.47 u^2 |rh|`.
///
/// Together below `2^-93.6 |rh|`; with `u |fl| <= 2.74 u^2 |rh|` for [`round_test`],
/// `eps = 2^-92 |rh|` is more than three times that. The reduction error `err` passes through unchanged,
/// as `|sin a - sin b| <= |a - b|`, and is added to `eps`.
fn sin_reduced(r: &Reduced, z: (f64, f64)) -> [f64; 3] {
    let s = series(z, &SIN_DD, &SIN_TAIL);
    let w = mul_dd(z, s);
    let (qh, ql) = mul_dd((r.rh, r.rl), w);
    let (fh, e) = two_sum(r.rh, qh);
    let fl = e + (r.rl + ql);
    [fh, fl, TWO_M92 * r.rh.abs() + r.err]
}

/// `cos r` as `fh + fl`, with the bound `eps` for [`round_test`].
///
/// `cos r = 1 + w` with `w = z C(z)`, `-0.3086 <= w <= 0`. The absolute errors:
///
/// - `C` within `2^-96` ([`series`]), times `z <= 0.6171`: `2^-96.69`;
/// - `z` within `6.01 u^2 rh^2` of `r^2`, times `|d(zC)/dz| <= 1/2`: below `2^-104`;
/// - `w = z C` by [`mul_dd`] with `x = 3.02`, `y = 1`: `16.1 u^2 zh |ch| <= 5 u^2`, and
///   `|wl| <= 5.05 u |wh| <= 1.56 u`;
/// - `1 + wh = fh + e` exactly (`two_sum`), and `e + wl`, below `2.56 u`: `2.56 u^2`.
///
/// Together below `2^-96.6`; with `u |fl| <= 2.56 u^2` for [`round_test`], `eps = 2^-95` is more than
/// three times that. The reduction error `err` is added, as `|cos a - cos b| <= |a - b|`.
fn cos_reduced(r: &Reduced, z: (f64, f64)) -> [f64; 3] {
    let c = series(z, &COS_DD, &COS_TAIL);
    let (wh, wl) = mul_dd(z, c);
    let (fh, e) = two_sum(1.0, wh);
    [fh, e + wl, TWO_M95 + r.err]
}

/// `-v` as a value and its bound: rounding is symmetric, so [`round_test`] gives the negated result.
fn neg([hi, lo, eps]: [f64; 3]) -> [f64; 3] {
    [-hi, -lo, eps]
}

/// `sin ax` and `cos ax` as values with bounds, from `sin r` and `cos r` by the quadrant.
fn by_quadrant(quadrant: u32, s: [f64; 3], c: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    match quadrant {
        0 => (s, c),
        1 => (c, neg(s)),
        2 => (neg(s), neg(c)),
        _ => (neg(c), s),
    }
}

/// `sin ax` for `2^-100 <= ax`, correctly rounded, or `None` for the fixed-point path.
pub(crate) fn sin(ax: f64) -> Option<f64> {
    let r = reduce(ax)?;
    let z = r.square();
    let value = match r.quadrant {
        0 => sin_reduced(&r, z),
        1 => cos_reduced(&r, z),
        2 => neg(sin_reduced(&r, z)),
        _ => neg(cos_reduced(&r, z)),
    };
    round_test(value)
}

/// `cos ax` for `2^-100 <= ax`, correctly rounded, or `None` for the fixed-point path.
pub(crate) fn cos(ax: f64) -> Option<f64> {
    let r = reduce(ax)?;
    let z = r.square();
    let value = match r.quadrant {
        0 => cos_reduced(&r, z),
        1 => neg(sin_reduced(&r, z)),
        2 => neg(cos_reduced(&r, z)),
        _ => sin_reduced(&r, z),
    };
    round_test(value)
}

/// `(sin ax, cos ax)` for `2^-100 <= ax` with one reduction, each correctly rounded or `None`.
pub(crate) fn sin_cos(ax: f64) -> (Option<f64>, Option<f64>) {
    let Some((s, c)) = sin_cos_parts(ax) else {
        return (None, None);
    };
    (round_test(s), round_test(c))
}

/// `sin ax` and `cos ax` before rounding, each as `[hi, lo, eps]`; the tests measure the error of
/// `hi + lo` against `eps`.
pub(crate) fn sin_cos_parts(ax: f64) -> Option<([f64; 3], [f64; 3])> {
    let r = reduce(ax)?;
    let z = r.square();
    Some(by_quadrant(
        r.quadrant,
        sin_reduced(&r, z),
        cos_reduced(&r, z),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_test_rejects_an_interval_across_a_midpoint() {
        let u = f64::EPSILON / 2.0;
        // 1 + u is the midpoint between 1 and its successor, 1 - u/2 between 1 and its predecessor.
        // With eps = u^2, `u + eps` would round back to `u` (a tie), which is why the bound must
        // exceed the true error by `u |lo|`; 4u^2 is exact here.
        assert_eq!(round_test([1.0, u, 4.0 * u * u]), None);
        assert_eq!(round_test([1.0, -u / 2.0, 4.0 * u * u]), None);
        assert_eq!(round_test([1.0, -u, u * u]), Some(1.0 - u));
        assert_eq!(round_test([1.0, u / 2.0, u * u]), Some(1.0));
        assert_eq!(round_test(neg([1.0, u / 2.0, u * u])), Some(-1.0));
        // Inside the interval but on both sides of a midpoint: a wider eps rejects it.
        assert_eq!(round_test([1.0, u / 2.0, u]), None);
    }

    #[test]
    fn reduction_is_exact_without_a_multiple() {
        let r = reduce(0.5).expect("in range");
        assert_eq!((r.quadrant, r.rh, r.rl, r.err), (0, 0.5, 0.0, 0.0));
        assert!(reduce(f64::from_bits(LIMIT.to_bits() + 1)).is_none());
        assert!(reduce(f64::NAN).is_none());
    }
}
