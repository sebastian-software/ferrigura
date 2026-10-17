//! The inverse trigonometric functions as fdlibm computes them in V8 (`src/base/ieee754.cc`):
//! `atan`, `atan2`, `asin` and `acos`. The multiply-adds follow V8's compiler (see [`crate::fused`]).

use crate::constants::{
    ASIN_PIO2_HI, ASIN_PIO2_LO, ASIN_PIO4_HI, ASIN_PS0, ASIN_PS1, ASIN_PS2, ASIN_PS3, ASIN_PS4,
    ASIN_PS5, ASIN_QS1, ASIN_QS2, ASIN_QS3, ASIN_QS4, ATAN_HI, ATAN_LO, ATAN_T, ATAN2_PI,
    ATAN2_PI_LO, ATAN2_PI_O_2, ATAN2_PI_O_4, PI_ASIN_ACOS,
};
use crate::fused::{mul_add, mul_sub, sub_mul};
use crate::words::{high_word, low_word, set_low_word};

/// `atan(x)`: the principal value of the arc tangent (V8's `ieee754::atan`).
#[must_use]
pub fn atan(x: f64) -> f64 {
    const ONE: f64 = 1.0;
    const HUGE: f64 = 1.0e300;
    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    let mut x = x;
    if ix >= 0x4410_0000 {
        // |x| >= 2^66
        let low = low_word(x);
        if ix > 0x7FF0_0000 || (ix == 0x7FF0_0000 && low != 0) {
            return x + x;
        }
        return if hx > 0 {
            ATAN_HI[3] + ATAN_LO[3]
        } else {
            -ATAN_HI[3] - ATAN_LO[3]
        };
    }
    let id: i32;
    if ix < 0x3FDC_0000 {
        // |x| < 0.4375
        if ix < 0x3E40_0000 && HUGE + x > ONE {
            // |x| < 2^-27
            return x;
        }
        id = -1;
    } else {
        x = x.abs();
        if ix < 0x3FF3_0000 {
            // |x| < 1.1875
            if ix < 0x3FE6_0000 {
                // 7/16 <= |x| < 11/16
                id = 0;
                x = mul_sub(2.0, x, ONE) / (2.0 + x);
            } else {
                // 11/16 <= |x| < 19/16
                id = 1;
                x = (x - ONE) / (x + ONE);
            }
        } else if ix < 0x4003_8000 {
            // |x| < 2.4375
            id = 2;
            x = (x - 1.5) / mul_add(1.5, x, ONE);
        } else {
            id = 3;
            x = -1.0 / x;
        }
    }
    let t = &ATAN_T;
    let z = x * x;
    let w = z * z;
    let a5 = mul_add(w, t[10], t[8]);
    let a4 = mul_add(w, a5, t[6]);
    let a3 = mul_add(w, a4, t[4]);
    let a2 = mul_add(w, a3, t[2]);
    let s1 = z * mul_add(w, a2, t[0]);
    let b4 = mul_add(w, t[9], t[7]);
    let b3 = mul_add(w, b4, t[5]);
    let b2 = mul_add(w, b3, t[3]);
    let s2 = w * mul_add(w, b2, t[1]);
    if id < 0 {
        sub_mul(x, x, s1 + s2)
    } else {
        let id = id as usize;
        let z = ATAN_HI[id] - (mul_sub(x, s1 + s2, ATAN_LO[id]) - x);
        if hx < 0 { -z } else { z }
    }
}

/// `atan2(y, x)`: the angle of the vector `(x, y)` from the positive x axis (V8's `ieee754::atan2`).
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    // V8 reads this from a volatile, so that the compiler cannot fold it. Adding it to these values
    // leaves them unchanged.
    const TINY: f64 = 1.0e-300;
    const ZERO: f64 = 0.0;
    let hx = high_word(x);
    let lx = low_word(x);
    let ix = hx & 0x7FFF_FFFF;
    let hy = high_word(y);
    let ly = low_word(y);
    let iy = hy & 0x7FFF_FFFF;
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    if (hx.wrapping_sub(0x3FF0_0000) as u32 | lx) == 0 {
        // x == 1
        return atan(y);
    }
    // m: bit 0 for the sign of y, bit 1 for the sign of x.
    let m = ((hy >> 31) & 1) | ((hx >> 30) & 2);
    if (iy as u32 | ly) == 0 {
        // y == +-0
        match m {
            0 | 1 => return y,
            2 => return ATAN2_PI + TINY,
            _ => return -ATAN2_PI - TINY,
        }
    }
    if (ix as u32 | lx) == 0 {
        // x == +-0, y != 0
        return if hy < 0 {
            -ATAN2_PI_O_2 - TINY
        } else {
            ATAN2_PI_O_2 + TINY
        };
    }
    if ix == 0x7FF0_0000 {
        // x is infinite
        if iy == 0x7FF0_0000 {
            return match m {
                0 => ATAN2_PI_O_4 + TINY,
                1 => -ATAN2_PI_O_4 - TINY,
                2 => mul_add(3.0, ATAN2_PI_O_4, TINY),
                _ => mul_sub(-3.0, ATAN2_PI_O_4, TINY),
            };
        }
        return match m {
            0 => ZERO,
            1 => -ZERO,
            2 => ATAN2_PI + TINY,
            _ => -ATAN2_PI - TINY,
        };
    }
    if iy == 0x7FF0_0000 {
        // y is infinite, x finite
        return if hy < 0 {
            -ATAN2_PI_O_2 - TINY
        } else {
            ATAN2_PI_O_2 + TINY
        };
    }
    // The exponents of x and y decide whether the atan can be shortcut.
    let k = (iy - ix) >> 20;
    let mut m = m;
    let z = if k > 60 {
        // |y/x| > 2^60
        m &= 1;
        mul_add(0.5, ATAN2_PI_LO, ATAN2_PI_O_2)
    } else if hx < 0 && k < -60 {
        // |y/x| < 2^-60 and x < 0
        0.0
    } else {
        atan((y / x).abs())
    };
    match m {
        0 => z,
        1 => -z,
        2 => ATAN2_PI - (z - ATAN2_PI_LO),
        _ => (z - ATAN2_PI_LO) - ATAN2_PI,
    }
}

/// The rational approximation shared by `asin` and `acos`: `(p, q)` with `p / q` the value of the
/// approximation of `z`.
#[inline]
fn asin_pq(z: f64) -> (f64, f64) {
    let w4 = mul_add(z, ASIN_PS5, ASIN_PS4);
    let w3 = mul_add(z, w4, ASIN_PS3);
    let w2 = mul_add(z, w3, ASIN_PS2);
    let w1 = mul_add(z, w2, ASIN_PS1);
    let w0 = mul_add(z, w1, ASIN_PS0);
    let p = z * w0;
    let v3 = mul_add(z, ASIN_QS4, ASIN_QS3);
    let v2 = mul_add(z, v3, ASIN_QS2);
    let v1 = mul_add(z, v2, ASIN_QS1);
    let q = mul_add(z, v1, 1.0);
    (p, q)
}

/// `acos(x)`: the principal value of the arc cosine, `NaN` for `|x| > 1` (V8's `ieee754::acos`).
#[must_use]
pub fn acos(x: f64) -> f64 {
    const ONE: f64 = 1.0;
    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix >= 0x3FF0_0000 {
        // |x| >= 1
        let lx = low_word(x);
        if ((ix - 0x3FF0_0000) as u32 | lx) == 0 {
            // |x| == 1
            return if hx > 0 {
                0.0
            } else {
                mul_add(2.0, ASIN_PIO2_LO, PI_ASIN_ACOS)
            };
        }
        return f64::NAN;
    }
    if ix < 0x3FE0_0000 {
        // |x| < 0.5
        if ix <= 0x3C60_0000 {
            return ASIN_PIO2_HI + ASIN_PIO2_LO;
        }
        let z = x * x;
        let (p, q) = asin_pq(z);
        let r = p / q;
        return ASIN_PIO2_HI - (x - sub_mul(ASIN_PIO2_LO, x, r));
    }
    if hx < 0 {
        // x <= -0.5
        let z = (ONE + x) * 0.5;
        let (p, q) = asin_pq(z);
        let s = z.sqrt();
        let r = p / q;
        let w = mul_sub(r, s, ASIN_PIO2_LO);
        return sub_mul(PI_ASIN_ACOS, 2.0, s + w);
    }
    // x >= 0.5
    let z = (ONE - x) * 0.5;
    let s = z.sqrt();
    let df = set_low_word(s, 0);
    let c = sub_mul(z, df, df) / (s + df);
    let (p, q) = asin_pq(z);
    let r = p / q;
    let w = mul_add(r, s, c);
    2.0 * (df + w)
}

/// `asin(x)`: the principal value of the arc sine, `NaN` for `|x| > 1` (V8's `ieee754::asin`).
#[must_use]
pub fn asin(x: f64) -> f64 {
    const ONE: f64 = 1.0;
    const HUGE: f64 = 1.000e+300;
    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix >= 0x3FF0_0000 {
        // |x| >= 1
        let lx = low_word(x);
        if ((ix - 0x3FF0_0000) as u32 | lx) == 0 {
            return mul_add(x, ASIN_PIO2_HI, x * ASIN_PIO2_LO);
        }
        return f64::NAN;
    }
    if ix < 0x3FE0_0000 {
        // |x| < 0.5
        let mut t = 0.0;
        if ix < 0x3E40_0000 {
            // |x| < 2^-27
            if HUGE + x > ONE {
                return x;
            }
        } else {
            t = x * x;
        }
        let (p, q) = asin_pq(t);
        let w = p / q;
        return mul_add(x, w, x);
    }
    // 0.5 <= |x| < 1
    let w = ONE - x.abs();
    let t = w * 0.5;
    let (p, q) = asin_pq(t);
    let s = t.sqrt();
    let t = if ix >= 0x3FEF_3333 {
        // |x| > 0.975
        let w = p / q;
        ASIN_PIO2_HI - mul_sub(2.0, mul_add(s, w, s), ASIN_PIO2_LO)
    } else {
        let w = set_low_word(s, 0);
        let c = sub_mul(t, w, w) / (s + w);
        let r = p / q;
        let p = mul_sub(2.0 * s, r, sub_mul(ASIN_PIO2_LO, 2.0, c));
        let q = sub_mul(ASIN_PIO4_HI, 2.0, w);
        ASIN_PIO4_HI - (p - q)
    };
    if hx > 0 { t } else { -t }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        assert_eq!(atan(0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(atan(-0.0).to_bits(), (-0.0f64).to_bits());
        assert_eq!(atan2(0.0, 1.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(atan2(-0.0, 1.0).to_bits(), (-0.0f64).to_bits());
        assert!(asin(1.5).is_nan());
        assert!(acos(-1.5).is_nan());
        assert_eq!(acos(1.0), 0.0);
        assert_eq!(asin(0.0).to_bits(), 0.0f64.to_bits());
    }
}
