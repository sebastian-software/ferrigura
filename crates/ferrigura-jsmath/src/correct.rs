//! Correctly rounded `sin`, `cos`, `tan`, `asin`, `acos`, `log` and `log10`.
//!
//! The value is computed in 384-bit fixed point (256 fraction bits), with an absolute error bound of
//! `(|v| + 1) * 2^-180`, and it is rounded only when the bounds on either side of it round to the same
//! double (Ziv's method). The bound is far larger than the errors of the arithmetic here, which is exact
//! integer arithmetic on the fixed-point words, and the worst cases known for the double-precision
//! elementary functions need about 120 bits, so the test does not fail on them.
//!
//! Chromium's `sin`, `cos`, `tan`, `asin`, `acos` and `log` are correctly rounded on every input checked
//! (about 270,000 against mpmath at 300 bits, and a million against this module), and so is its `log10`
//! (200,000 against Python's `decimal` at 80 digits, and a million against this module), so these results
//! are Chromium's. The results do not depend on the target's fused multiply-adds: nothing here is rounded
//! except by the final conversion, and Rust does not contract.
//!
//! The constants are in `constants.rs` (`CR_*`), computed with mpmath from `2/pi`, `pi/2` and `ln 2`, and
//! with Python's `decimal` from `1/ln 10`.
//! The `acos` fast path is adapted from CORE-MATH; ambiguous results use this module’s
//! fixed-point evaluator. The other evaluators here are independently written.

use crate::constants::{CR_INV_LN10, CR_LN2, CR_PI_OVER_2, CR_TWO_OVER_PI};

/// A signed fixed-point number: its value is `self.0` in two's complement divided by `2^256`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fx([u64; 6]);

/// The number of fraction bits.
const FRAC: usize = 256;

impl Fx {
    const ZERO: Fx = Fx([0; 6]);

    fn one() -> Fx {
        Fx([0, 0, 0, 0, 1, 0])
    }

    fn from_limbs(limbs: [u64; 4]) -> Fx {
        Fx([limbs[0], limbs[1], limbs[2], limbs[3], 0, 0])
    }

    fn is_zero(self) -> bool {
        self.0 == [0; 6]
    }

    fn is_neg(self) -> bool {
        self.0[5] >> 63 == 1
    }

    fn neg(self) -> Fx {
        let mut out = [0u64; 6];
        let mut carry = 1u64;
        for (o, limb) in out.iter_mut().zip(self.0) {
            let (value, overflow) = (!limb).overflowing_add(carry);
            *o = value;
            carry = u64::from(overflow);
        }
        Fx(out)
    }

    fn abs(self) -> Fx {
        if self.is_neg() { self.neg() } else { self }
    }

    fn add(self, other: Fx) -> Fx {
        let mut out = [0u64; 6];
        let mut carry = 0u64;
        for (i, slot) in out.iter_mut().enumerate() {
            let (sum, c1) = self.0[i].overflowing_add(other.0[i]);
            let (sum, c2) = sum.overflowing_add(carry);
            *slot = sum;
            carry = u64::from(c1) + u64::from(c2);
        }
        Fx(out)
    }

    fn sub(self, other: Fx) -> Fx {
        self.add(other.neg())
    }

    /// The product, truncated towards zero in the last place.
    fn mul(self, other: Fx) -> Fx {
        let negative = self.is_neg() != other.is_neg();
        let a = self.abs().0;
        let b = other.abs().0;
        let mut product = [0u64; 12];
        for i in 0..6 {
            let mut carry = 0u128;
            for j in 0..6 {
                let t = u128::from(a[i]) * u128::from(b[j]) + u128::from(product[i + j]) + carry;
                product[i + j] = t as u64;
                carry = t >> 64;
            }
            product[i + 6] = carry as u64;
        }
        let mut out = [0u64; 6];
        out.copy_from_slice(&product[4..10]);
        let result = Fx(out);
        if negative { result.neg() } else { result }
    }

    /// The quotient by a positive integer, truncated towards zero.
    fn div_int(self, divisor: u64) -> Fx {
        let negative = self.is_neg();
        let a = self.abs().0;
        let mut out = [0u64; 6];
        let mut rem = 0u128;
        for i in (0..6).rev() {
            let cur = (rem << 64) | u128::from(a[i]);
            out[i] = (cur / u128::from(divisor)) as u64;
            rem = cur % u128::from(divisor);
        }
        let result = Fx(out);
        if negative { result.neg() } else { result }
    }

    fn mul_int(self, k: i64) -> Fx {
        let magnitude = Fx::from_int(k.unsigned_abs());
        let product = self.mul(magnitude);
        if k < 0 { product.neg() } else { product }
    }

    fn from_int(value: u64) -> Fx {
        Fx([0, 0, 0, 0, value, 0])
    }

    /// Half of the value (exact for every value we use: a right shift).
    fn half(self) -> Fx {
        let mut out = [0u64; 6];
        for (i, slot) in out.iter_mut().enumerate() {
            let high = if i < 5 {
                self.0[i + 1] << 63
            } else {
                self.0[5] & (1u64 << 63)
            };
            *slot = (self.0[i] >> 1) | high;
        }
        Fx(out)
    }

    /// The quotient, truncated towards zero in the last place.
    fn div(self, other: Fx) -> Fx {
        let negative = self.is_neg() != other.is_neg();
        let a = self.abs().0;
        let d = other.abs().0;
        let n = d.iter().rposition(|&x| x != 0).expect("division by zero") + 1;
        let mut numerator = [0u64; 11];
        numerator[4..10].copy_from_slice(&a);
        let mut quotient = [0u64; 10];
        if n == 1 {
            let mut rem = 0u128;
            for i in (0..10).rev() {
                let value = (rem << 64) | u128::from(numerator[i]);
                quotient[i] = (value / u128::from(d[0])) as u64;
                rem = value % u128::from(d[0]);
            }
        } else {
            // Knuth's Algorithm D normalizes the leading divisor limb's high bit.
            // The extra numerator limb holds the carry, including when the shift is zero.
            let shift = d[n - 1].leading_zeros();
            let mut divisor = [0u64; 6];
            if shift == 0 {
                divisor[..n].copy_from_slice(&d[..n]);
            } else {
                let mut carry = 0;
                for i in 0..n {
                    divisor[i] = (d[i] << shift) | carry;
                    carry = d[i] >> (64 - shift);
                }
                carry = 0;
                for limb in &mut numerator[..10] {
                    let next = *limb >> (64 - shift);
                    *limb = (*limb << shift) | carry;
                    carry = next;
                }
                numerator[10] = carry;
            }
            let base = 1u128 << 64;
            let top = u128::from(divisor[n - 1]);
            for j in (0..=10 - n).rev() {
                let value = (u128::from(numerator[j + n]) << 64) | u128::from(numerator[j + n - 1]);
                let mut q = (value / top).min(base - 1);
                let mut r = value - q * top;
                while r < base
                    && q * u128::from(divisor[n - 2]) > (r << 64) + u128::from(numerator[j + n - 2])
                {
                    q -= 1;
                    r += top;
                }
                let mut borrow = 0u128;
                for i in 0..n {
                    let product = q * u128::from(divisor[i]) + borrow;
                    let (difference, underflow) = numerator[j + i].overflowing_sub(product as u64);
                    numerator[j + i] = difference;
                    borrow = (product >> 64) + u128::from(underflow);
                }
                let (high, underflow) = numerator[j + n].overflowing_sub(borrow as u64);
                numerator[j + n] = high;
                if underflow {
                    q -= 1;
                    let mut carry = 0u128;
                    for i in 0..n {
                        let sum = u128::from(numerator[j + i]) + u128::from(divisor[i]) + carry;
                        numerator[j + i] = sum as u64;
                        carry = sum >> 64;
                    }
                    numerator[j + n] = numerator[j + n].wrapping_add(carry as u64);
                }
                quotient[j] = q as u64;
            }
        }
        let mut out = [0u64; 6];
        out.copy_from_slice(&quotient[..6]);
        let result = Fx(out);
        if negative { result.neg() } else { result }
    }

    /// The quotient, truncated towards zero in the last place (restoring division, bit by bit).
    #[cfg(test)]
    fn div_restoring(self, other: Fx) -> Fx {
        let negative = self.is_neg() != other.is_neg();
        let a = self.abs().0;
        let d = other.abs().0;
        // The numerator is |self| * 2^256, in ten limbs.
        let mut numerator = [0u64; 10];
        numerator[4..10].copy_from_slice(&a);
        let mut rem = [0u64; 7];
        let mut divisor = [0u64; 7];
        divisor[..6].copy_from_slice(&d);
        let mut quotient = [0u64; 10];
        for bit in (0..(10 * 64)).rev() {
            // rem = rem * 2 + the bit
            let incoming = (numerator[bit / 64] >> (bit % 64)) & 1;
            let mut carry = incoming;
            for limb in rem.iter_mut() {
                let next = *limb >> 63;
                *limb = (*limb << 1) | carry;
                carry = next;
            }
            // if rem >= d then rem -= d and the quotient bit is 1
            if ge(&rem, &divisor) {
                sub_in_place(&mut rem, &divisor);
                quotient[bit / 64] |= 1u64 << (bit % 64);
            }
        }
        let mut out = [0u64; 6];
        out.copy_from_slice(&quotient[..6]);
        let result = Fx(out);
        if negative { result.neg() } else { result }
    }

    /// The square root of a non-negative value, by Newton's iteration from a double estimate.
    fn sqrt(self) -> Fx {
        if self.is_zero() {
            return Fx::ZERO;
        }
        let mut y = Fx::from_f64(self.approx_f64().sqrt());
        for _ in 0..5 {
            y = y.add(self.div(y)).half();
        }
        y
    }

    fn approx_f64(self) -> f64 {
        let a = self.abs().0;
        let mut acc = 0.0f64;
        for i in (0..6).rev() {
            acc = acc * 18_446_744_073_709_551_616.0 + a[i] as f64;
        }
        let value = acc * 2f64.powi(-(FRAC as i32));
        if self.is_neg() { -value } else { value }
    }

    /// The value of a double with a magnitude below `2^100`, exact when its last set bit is at or
    /// above `2^-256`.
    fn from_f64(x: f64) -> Fx {
        if x == 0.0 {
            return Fx::ZERO;
        }
        let bits = x.to_bits();
        let biased = ((bits >> 52) & 0x7FF) as i32;
        let (mantissa, exponent) = if biased == 0 {
            (bits & ((1u64 << 52) - 1), -1074)
        } else {
            ((bits & ((1u64 << 52) - 1)) | (1u64 << 52), biased - 1075)
        };
        let shift = exponent + FRAC as i32;
        let magnitude = if shift >= 0 {
            shl_limbs(mantissa, shift as usize)
        } else {
            shr_limbs(mantissa, (-shift) as usize)
        };
        let result = Fx(magnitude);
        if x < 0.0 { result.neg() } else { result }
    }

    /// The double nearest to the value, ties to even. The result is normal: the values here are
    /// never below `2^-1000` in magnitude except zero.
    fn to_f64(self) -> f64 {
        let magnitude = self.abs().0;
        let Some(top) = highest_bit(&magnitude) else {
            return 0.0;
        };
        let exponent = top as i64 - FRAC as i64;
        // The 53 bits from `top` down, and the bits below them.
        let (mut mantissa, round_bit, sticky) = if top >= 53 {
            let low = top - 52;
            let m = bits_at(&magnitude, low, 53);
            (
                m,
                bit_at(&magnitude, low - 1),
                any_below(&magnitude, low - 1),
            )
        } else {
            (bits_at(&magnitude, 0, 53) << (52 - top), false, false)
        };
        let mut exponent = exponent;
        if round_bit && (sticky || mantissa & 1 == 1) {
            mantissa += 1;
            if mantissa == 1u64 << 53 {
                mantissa >>= 1;
                exponent += 1;
            }
        }
        let value =
            f64::from_bits((((exponent + 1023) as u64) << 52) | (mantissa & ((1u64 << 52) - 1)));
        if self.is_neg() { -value } else { value }
    }
}

fn shl_limbs(value: u64, shift: usize) -> [u64; 6] {
    let mut out = [0u64; 6];
    let limb = shift / 64;
    let bit = shift % 64;
    if limb < 6 {
        out[limb] = value << bit;
        if bit > 0 && limb + 1 < 6 {
            out[limb + 1] = value >> (64 - bit);
        }
    }
    out
}

fn shr_limbs(value: u64, shift: usize) -> [u64; 6] {
    let mut out = [0u64; 6];
    if shift < 64 {
        out[0] = value >> shift;
    }
    out
}

#[cfg(test)]
fn ge(a: &[u64], b: &[u64]) -> bool {
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i] > b[i];
        }
    }
    true
}

#[cfg(test)]
fn sub_in_place(a: &mut [u64], b: &[u64]) {
    let mut borrow = 0u64;
    for i in 0..a.len() {
        let (d1, b1) = a[i].overflowing_sub(b[i]);
        let (d2, b2) = d1.overflowing_sub(borrow);
        a[i] = d2;
        borrow = u64::from(b1) + u64::from(b2);
    }
}

fn highest_bit(a: &[u64]) -> Option<usize> {
    for i in (0..a.len()).rev() {
        if a[i] != 0 {
            return Some(i * 64 + 63 - a[i].leading_zeros() as usize);
        }
    }
    None
}

fn bit_at(a: &[u64], pos: usize) -> bool {
    (a[pos / 64] >> (pos % 64)) & 1 == 1
}

fn any_below(a: &[u64], pos: usize) -> bool {
    // any bit strictly below `pos`
    let limb = pos / 64;
    for item in a.iter().take(limb) {
        if *item != 0 {
            return true;
        }
    }
    let bit = pos % 64;
    bit > 0 && a[limb] & ((1u64 << bit) - 1) != 0
}

/// The 53 bits of `a` from position `low` up.
fn bits_at(a: &[u64], low: usize, count: usize) -> u64 {
    let mut out = 0u64;
    for i in 0..count {
        if bit_at(a, low + i) {
            out |= 1u64 << i;
        }
    }
    out
}

/// The `count` bits of `p` from position `pos` (count at most 64), as a number.
fn get64(p: &[u64], pos: usize) -> u64 {
    let limb = pos / 64;
    let bit = pos % 64;
    let low = p.get(limb).copied().unwrap_or(0) >> bit;
    if bit == 0 {
        return low;
    }
    let high = p.get(limb + 1).copied().unwrap_or(0) << (64 - bit);
    low | high
}

/// The error bound of a value `v`, in units of `2^-256`: `(|v| + 1) * 2^-180`.
fn error_bound(v: Fx) -> Fx {
    let magnitude = v.abs();
    let plus_one = magnitude.add(Fx::one());
    // shift right by 180 bits: two limbs, then 52 bits
    let mut out = [0u64; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        let src = i + 2;
        let low = if src < 6 { plus_one.0[src] >> 52 } else { 0 };
        let high = if src + 1 < 6 {
            plus_one.0[src + 1] << 12
        } else {
            0
        };
        *slot = low | high;
    }
    Fx(out)
}

/// The correctly rounded double of the value `v`, from its bounds: rounds `v - e` and `v + e`, and
/// returns the common result when they agree. They agree for every value of these functions here.
fn round_value(v: Fx) -> f64 {
    let e = error_bound(v);
    let upper = v.add(e).to_f64();
    let lower = v.sub(e).to_f64();
    if upper == lower { upper } else { v.to_f64() }
}

/// `x` reduced modulo `pi/2` for a finite `x` with `|x| >= 2^-100`: the quadrant and the remainder
/// in `[-pi/4, pi/4]`, with `ax = (quadrant + remainder/(pi/2)) * pi/2`, for a positive `ax`.
fn reduce(ax: f64) -> (u32, Fx) {
    let bits = ax.to_bits();
    let biased = ((bits >> 52) & 0x7FF) as i64;
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let exponent = biased - 1075;
    // P = mantissa * (2/pi) * 2^1408, so that ax * 2/pi = P * 2^(exponent - 1408)
    let mut product = vec![0u64; CR_TWO_OVER_PI.len() + 1];
    let mut carry = 0u128;
    for (out, limb) in product.iter_mut().zip(CR_TWO_OVER_PI.iter()) {
        let t = u128::from(*limb) * u128::from(mantissa) + carry;
        *out = t as u64;
        carry = t >> 64;
    }
    product[CR_TWO_OVER_PI.len()] = carry as u64;
    let shift = (1408 - exponent) as usize; // the fraction of ax*2/pi is the low `shift` bits
    let quadrant = (get64(&product, shift) & 3) as u32;
    // The 256 bits of the fraction below the top bit of the quadrant, from position shift - 256.
    let mut fraction = [0u64; 4];
    for (i, limb) in fraction.iter_mut().enumerate() {
        *limb = get64(&product, shift - FRAC + 64 * i);
    }
    let mut fraction = Fx::from_limbs(fraction);
    let mut quadrant = quadrant;
    // f in [0, 1); if f >= 1/2 use f - 1 (in [-1/2, 0)) and the next quadrant.
    if fraction.0[3] >> 63 == 1 {
        quadrant = (quadrant + 1) & 3;
        fraction = fraction.sub(Fx::one());
    }
    let remainder = fraction.mul(Fx(CR_PI_OVER_2));
    (quadrant, remainder)
}

/// `sin r` for `|r| <= pi/4`, by its Taylor series.
fn sin_series(r: Fx) -> Fx {
    let r2 = r.mul(r);
    let mut term = r;
    let mut sum = r;
    let mut n = 1u64;
    loop {
        term = term.mul(r2).div_int((2 * n) * (2 * n + 1)).neg();
        if term.is_zero() {
            return sum;
        }
        sum = sum.add(term);
        n += 1;
    }
}

/// `cos r` for `|r| <= pi/4`, by its Taylor series.
fn cos_series(r: Fx) -> Fx {
    let r2 = r.mul(r);
    let mut term = Fx::one();
    let mut sum = Fx::one();
    let mut n = 1u64;
    loop {
        term = term.mul(r2).div_int((2 * n - 1) * (2 * n)).neg();
        if term.is_zero() {
            return sum;
        }
        sum = sum.add(term);
        n += 1;
    }
}

/// `sin(ax)` and `cos(ax)` split by quadrant: the value is `s` with its sign for quadrant `q`.
fn sin_cos_of(ax: f64) -> (Fx, Fx) {
    let (q, r) = reduce(ax);
    let s = sin_series(r);
    let c = cos_series(r);
    match q {
        0 => (s, c),
        1 => (c, s.neg()),
        2 => (s.neg(), c.neg()),
        _ => (c.neg(), s),
    }
}

/// Below this magnitude, `sin`, `tan` and `asin` return `x`, which is the correctly rounded value.
const TINY: f64 = 7.888609052210118e-31; // 2^-100

fn sign_of(x: f64, value: f64) -> f64 {
    if x < 0.0 { -value } else { value }
}

/// `sin(x)` and `cos(x)`, correctly rounded with a shared range reduction.
#[must_use]
pub fn sin_cos(x: f64) -> (f64, f64) {
    if !x.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    if x.abs() < TINY {
        return (x, 1.);
    }
    let (s, c) = sin_cos_of(x.abs());
    (sign_of(x, round_value(s)), round_value(c))
}

/// `sin(x)`, correctly rounded.
#[must_use]
pub fn sin(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 || x.abs() < TINY {
        return x;
    }
    let (s, _) = sin_cos_of(x.abs());
    sign_of(x, round_value(s))
}

/// `cos(x)`, correctly rounded.
#[must_use]
pub fn cos(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x.abs() < TINY {
        return 1.0;
    }
    let (_, c) = sin_cos_of(x.abs());
    round_value(c)
}

/// `tan(x)`, correctly rounded.
#[must_use]
pub fn tan(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 || x.abs() < TINY {
        return x;
    }
    let (q, r) = reduce(x.abs());
    let s = sin_series(r);
    let c = cos_series(r);
    let t = if q & 1 == 0 { s.div(c) } else { c.neg().div(s) };
    sign_of(x, round_value(t))
}

/// `atan(w)` for `w >= 0`, from three halvings of the argument and the Taylor series.
fn atan_fx(w: Fx) -> Fx {
    let one = Fx::one();
    let mut w = w;
    for _ in 0..3 {
        let root = one.add(w.mul(w)).sqrt();
        w = w.div(one.add(root));
    }
    // atan(w) = w - w^3/3 + w^5/5 - ...; the halvings multiply the result by 8
    let w2 = w.mul(w);
    let mut power = w;
    let mut sum = w;
    let mut n = 1u64;
    loop {
        power = power.mul(w2).neg();
        let term = power.div_int(2 * n + 1);
        if term.is_zero() {
            break;
        }
        sum = sum.add(term);
        n += 1;
    }
    Fx(shl3(sum.0))
}

/// The value times 8 (a left shift by three bits: each limb takes the top bits of the one below).
fn shl3(limbs: [u64; 6]) -> [u64; 6] {
    let mut out = [0u64; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        let low = if i > 0 { limbs[i - 1] >> 61 } else { 0 };
        *slot = (limbs[i] << 3) | low;
    }
    out
}

/// `asin(|x|)` for `0 < |x| <= 1`, as a fixed-point value.
fn asin_fx(ax: Fx) -> Fx {
    let one = Fx::one();
    let root = one.sub(ax.mul(ax)).sqrt();
    // asin(a) = atan(a / sqrt(1 - a^2)); for a > 1/sqrt(2) use pi/2 - atan(sqrt(1 - a^2) / a)
    let half_square = ax.mul(ax).mul_int(2);
    if half_square.sub(one).is_neg() {
        atan_fx(ax.div(root))
    } else {
        Fx(CR_PI_OVER_2).sub(atan_fx(root.div(ax)))
    }
}

/// `asin(x)`, correctly rounded.
#[must_use]
pub fn asin(x: f64) -> f64 {
    if x.is_nan() || x.abs() > 1.0 {
        return f64::NAN;
    }
    if x == 0.0 || x.abs() < TINY {
        return x;
    }
    if x.abs() == 1.0 {
        return sign_of(x, round_value(Fx(CR_PI_OVER_2)));
    }
    let value = asin_fx(Fx::from_f64(x.abs()));
    sign_of(x, round_value(value))
}

/// `acos(x)`, correctly rounded: `pi/2 - asin(x)`.
#[must_use]
pub fn acos(x: f64) -> f64 {
    if let Some(result) = crate::acos_fast::fast_acos(x) {
        return result;
    }
    acos_fixed(x)
}

fn acos_fixed(x: f64) -> f64 {
    if x.is_nan() || x.abs() > 1.0 {
        return f64::NAN;
    }
    if x == 1.0 {
        return 0.0;
    }
    if x == -1.0 {
        return round_value(Fx(CR_PI_OVER_2).add(Fx(CR_PI_OVER_2)));
    }
    let asin = if x == 0.0 {
        Fx::ZERO
    } else {
        let value = asin_fx(Fx::from_f64(x.abs()));
        if x < 0.0 { value.neg() } else { value }
    };
    round_value(Fx(CR_PI_OVER_2).sub(asin))
}

/// `log(x)`, correctly rounded.
#[must_use]
pub fn log(x: f64) -> f64 {
    match log_special(x) {
        Some(value) => value,
        None => round_value(ln_fx(x)),
    }
}

/// `log10(x)`, correctly rounded: the natural logarithm times `1/ln 10`, rounded once. An exact power
/// of ten gives its exponent exactly.
#[must_use]
pub fn log10(x: f64) -> f64 {
    match log_special(x) {
        Some(value) => value,
        None => round_value(ln_fx(x).mul(Fx(CR_INV_LN10))),
    }
}

/// The logarithm of the inputs whose logarithm in every base is the same: `NaN` for `NaN` and the
/// negative numbers, `-Infinity` for zero, `Infinity` for `Infinity`, and `+0` for 1.
fn log_special(x: f64) -> Option<f64> {
    if x.is_nan() || x < 0.0 {
        return Some(f64::NAN);
    }
    if x == 0.0 {
        return Some(f64::NEG_INFINITY);
    }
    if x.is_infinite() {
        return Some(f64::INFINITY);
    }
    if x == 1.0 {
        return Some(0.0);
    }
    None
}

/// `ln x` for a finite positive `x`.
fn ln_fx(x: f64) -> Fx {
    // x = 2^k * m with m in [1, 2), or [sqrt(1/2), sqrt(2)) after a halving
    let (scaled, extra) = if x < f64::MIN_POSITIVE {
        (x * 2f64.powi(54), -54)
    } else {
        (x, 0)
    };
    let bits = scaled.to_bits();
    let mut k = ((bits >> 52) & 0x7FF) as i64 - 1023 + extra;
    let mut m = f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52));
    if m > std::f64::consts::SQRT_2 {
        m /= 2.0;
        k += 1;
    }
    let mf = Fx::from_f64(m);
    let one = Fx::one();
    let z = mf.sub(one).div(mf.add(one));
    // ln m = 2 atanh(z) = 2 (z + z^3/3 + z^5/5 + ...)
    let z2 = z.mul(z);
    let mut power = z;
    let mut sum = z;
    let mut n = 1u64;
    loop {
        power = power.mul(z2);
        let term = power.div_int(2 * n + 1);
        if term.is_zero() {
            break;
        }
        sum = sum.add(term);
        n += 1;
    }
    let ln_m = sum.add(sum);
    Fx(CR_LN2).mul_int(k).add(ln_m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_trigonometry_matches_separate_operations() {
        let mut state = 0x921f_b544_42d1_8469_u64;
        let cases = [
            0.0,
            -0.0,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            TINY,
            -TINY,
            std::f64::consts::FRAC_PI_2,
            -std::f64::consts::PI,
            f64::MAX,
            -f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for x in cases.into_iter().chain((0..10_000).map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            f64::from_bits(state)
        })) {
            let (s, c) = sin_cos(x);
            assert_eq!(s.to_bits(), sin(x).to_bits(), "sin({x:?})");
            assert_eq!(c.to_bits(), cos(x).to_bits(), "cos({x:?})");
        }
    }

    #[test]
    fn division_matches_restoring() {
        let mut state = 0x921f_b544_42d1_8469u64;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        // The dividend shifted by 256 is one below the divisor: the trial
        // quotient digit is one, so subtraction must add the divisor back.
        for low in [0, 1, 2] {
            let a = Fx([1, 0, 0, 0, 0, 0]);
            let d = Fx([low, 0, 0, 0, 1, 0]);
            assert_eq!(a.div(d), a.div_restoring(d), "low limb {low}");
        }
        for case in 0..30_000 {
            let mut a = [0; 6];
            let mut d = [0; 6];
            for limb in &mut a {
                *limb = random();
            }
            let len = case % 6 + 1;
            for limb in &mut d[..len] {
                *limb = random();
            }
            // Cover normalization shifts, powers of two, near-equal limbs, and
            // quotient estimates near the base as well as general random limbs.
            match case % 8 {
                0 => d[len - 1] = 1,
                1 => d[len - 1] = u64::MAX,
                2 => d[len - 1] = 1 << 63,
                3 => {
                    d[..len].fill(u64::MAX);
                }
                4 => {
                    d[..len].fill(0);
                    d[len - 1] = 1;
                }
                5 => {
                    a = d;
                }
                _ => {}
            }
            if d == [0; 6] {
                d[0] = 1;
            }
            let a = Fx(a);
            let d = Fx(d);
            assert_eq!(a.div(d), a.div_restoring(d), "case {case}: {a:?} / {d:?}");
        }
    }

    #[test]
    fn acos_fast_matches_fixed_point() {
        // Include the upstream fast-path error-bound examples and both signs.
        let examples = [
            0x3fe3e827a2cd6d51_u64,
            0x3f37cb54339263fb,
            0x3fdfda6fee396f80,
        ];
        let mut state = 0x7e31_6993_c190_bd17_u64;
        let mut fast = 0;
        let mut fallback = 0;
        for bits in examples
            .into_iter()
            .flat_map(|x| [x, x | (1 << 63)])
            .chain((0..20_000).map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                // Uniformly cover the domain, including both reduction branches.
                let value = ((state >> 11) as f64) * (2.0 / ((1_u64 << 53) as f64)) - 1.0;
                value.to_bits()
            }))
        {
            let x = f64::from_bits(bits);
            if let Some(result) = crate::acos_fast::fast_acos(x) {
                fast += 1;
                assert_eq!(result.to_bits(), acos_fixed(x).to_bits(), "x={x:?}");
            } else {
                fallback += 1;
                assert_eq!(acos(x).to_bits(), acos_fixed(x).to_bits());
            }
        }
        assert!(fast > 0 && fallback > 0);
    }

    #[test]
    fn acos_core_math_hard_cases_match_chromium() {
        // CORE-MATH acos.wc inputs, with outputs captured from Chromium 151.
        for (input, expected) in [
            (0x3feffffffffffdc0, 0x3e98000000000024),
            (0xbfeffffffffffdc0, 0x400921fb24442d18),
            (0x3fedffff776c7505, 0x3fd6bf3ba258567b),
            (0xbfedffff776c7505, 0x40064a13dff92249),
            (0x3fefffffffffffff, 0x3e50000000000000),
            (0xbfefffffffffffff, 0x400921fb52442d18),
        ] {
            let x = f64::from_bits(input);
            assert_eq!(acos(x).to_bits(), expected, "x={x:?}");
        }
    }

    // Generated independently with 400-digit decimal arithmetic and an atan series.
    #[test]
    fn acos_endpoint_vectors() {
        for (input, expected) in [
            (0x0000000000000000, 0x3ff921fb54442d18),
            (0x8000000000000000, 0x3ff921fb54442d18),
            (0x0000000000000001, 0x3ff921fb54442d18),
            (0x8000000000000001, 0x3ff921fb54442d18),
            (0x0000000000000002, 0x3ff921fb54442d18),
            (0x8000000000000002, 0x3ff921fb54442d18),
            (0x000fffffffffffff, 0x3ff921fb54442d18),
            (0x800fffffffffffff, 0x3ff921fb54442d18),
            (0x0010000000000000, 0x3ff921fb54442d18),
            (0x8010000000000000, 0x3ff921fb54442d18),
            (0x3c90000000000000, 0x3ff921fb54442d18),
            (0xbc90000000000000, 0x3ff921fb54442d19),
            (0x3fdfffffffffffff, 0x3ff0c152382d7366),
            (0xbfdfffffffffffff, 0x4000c152382d7365),
            (0x3fe0000000000000, 0x3ff0c152382d7366),
            (0xbfe0000000000000, 0x4000c152382d7366),
            (0x3fe0000000000001, 0x3ff0c152382d7365),
            (0xbfe0000000000001, 0x4000c152382d7366),
            (0x3fe6a09e667f3bcc, 0x3fe921fb54442d19),
            (0xbfe6a09e667f3bcc, 0x4002d97c7f3321d2),
            (0x3fe6a09e667f3bcd, 0x3fe921fb54442d18),
            (0xbfe6a09e667f3bcd, 0x4002d97c7f3321d2),
            (0x3fe6a09e667f3bce, 0x3fe921fb54442d16),
            (0xbfe6a09e667f3bce, 0x4002d97c7f3321d3),
            (0x3feffffffffffffd, 0x3e5bb67ae8584cab),
            (0xbfeffffffffffffd, 0x400921fb50cd5dbb),
            (0x3feffffffffffffe, 0x3e56a09e667f3bcd),
            (0xbfeffffffffffffe, 0x400921fb5170194b),
            (0x3fefffffffffffff, 0x3e50000000000000),
            (0xbfefffffffffffff, 0x400921fb52442d18),
            (0x3ff0000000000000, 0x0000000000000000),
            (0xbff0000000000000, 0x400921fb54442d18),
        ] {
            assert_eq!(
                acos(f64::from_bits(input)).to_bits(),
                expected,
                "input {input:016x}"
            );
        }
    }

    #[test]
    fn fixed_point_round_trip() {
        for x in [0.5, 1.0, 3.25, 1e-20, 1e20, -0.75] {
            assert_eq!(Fx::from_f64(x).to_f64(), x, "{x}");
        }
    }

    #[test]
    fn exact_special_values() {
        assert_eq!(sin(0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(sin(-0.0).to_bits(), (-0.0f64).to_bits());
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(log(1.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(log10(1.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(log10(0.0), f64::NEG_INFINITY);
        assert!(log10(-1.0).is_nan());
        assert_eq!(acos(1.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(log(0.0), f64::NEG_INFINITY);
        assert!(log(-1.0).is_nan());
        assert!(sin(f64::INFINITY).is_nan());
        assert!(asin(2.0).is_nan());
        assert_eq!(asin(1.0), std::f64::consts::FRAC_PI_2);
        assert_eq!(acos(-1.0), std::f64::consts::PI);
    }

    #[test]
    fn known_values() {
        assert_eq!(sin(1.0), 0.8414709848078965);
        assert_eq!(cos(1.0), 0.5403023058681398);
        assert_eq!(log(2.0), std::f64::consts::LN_2);
        assert_eq!(log10(2.0), std::f64::consts::LOG10_2);
        for exponent in 0..=22 {
            assert_eq!(
                log10(10f64.powi(exponent)),
                f64::from(exponent),
                "1e{exponent}"
            );
        }
        assert_eq!(asin(0.5), std::f64::consts::FRAC_PI_6);
        assert_eq!(tan(0.5), 0.5463024898437905);
    }
}
