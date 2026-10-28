//! `sin`, `cos` and `tan` as fdlibm computes them in V8 (`src/base/ieee754.cc`).
//!
//! V8 uses these for `Math.sin`, `Math.cos` and `Math.tan` in builds without
//! `V8_USE_LIBM_TRIG_FUNCTIONS`, which is how Node is built. The multiply-adds follow V8's compiler
//! (see [`crate::fused`]).

use crate::constants::{
    COS_C1, COS_C2, COS_C3, COS_C4, COS_C5, COS_C6, SIN_S1, SIN_S2, SIN_S3, SIN_S4, SIN_S5, SIN_S6,
    TAN_ONE, TAN_PIO4, TAN_PIO4LO, TAN_T,
};
use crate::fused::{mul_add, mul_sub, sub_mul};
use crate::rem_pio2::rem_pio2;
use crate::words::{from_words, high_word, low_word, set_low_word};

/// `sin(x)` (V8's `ieee754::sin`).
#[must_use]
pub fn sin(x: f64) -> f64 {
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        return kernel_sin(x, 0.0, 0);
    }
    if ix >= 0x7FF0_0000 {
        return f64::NAN;
    }
    let mut y = [0.0f64; 2];
    let n = rem_pio2(x, &mut y);
    match n & 3 {
        0 => kernel_sin(y[0], y[1], 1),
        1 => kernel_cos(y[0], y[1]),
        2 => -kernel_sin(y[0], y[1], 1),
        _ => -kernel_cos(y[0], y[1]),
    }
}

/// `cos(x)` (V8's `ieee754::cos`).
#[must_use]
pub fn cos(x: f64) -> f64 {
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        return kernel_cos(x, 0.0);
    }
    if ix >= 0x7FF0_0000 {
        return f64::NAN;
    }
    let mut y = [0.0f64; 2];
    let n = rem_pio2(x, &mut y);
    match n & 3 {
        0 => kernel_cos(y[0], y[1]),
        1 => -kernel_sin(y[0], y[1], 1),
        2 => -kernel_cos(y[0], y[1]),
        _ => kernel_sin(y[0], y[1], 1),
    }
}

/// `tan(x)` (V8's `ieee754::tan`), with `x` in radians.
#[must_use]
pub fn tan(x: f64) -> f64 {
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        return kernel_tan(x, 0.0, 1);
    }
    if ix >= 0x7FF0_0000 {
        return f64::NAN;
    }
    let mut y = [0.0f64; 2];
    let n = rem_pio2(x, &mut y);
    kernel_tan(y[0], y[1], 1 - ((n & 1) << 1))
}

/// `__kernel_cos`: the cosine on `[-pi/4, pi/4]`, for `x + y`.
fn kernel_cos(x: f64, y: f64) -> f64 {
    const ONE: f64 = 1.0;
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix < 0x3E40_0000 && (x as i32) == 0 {
        return ONE;
    }
    let z = x * x;
    let c5 = mul_add(z, COS_C6, COS_C5);
    let c4 = mul_add(z, c5, COS_C4);
    let c3 = mul_add(z, c4, COS_C3);
    let c2 = mul_add(z, c3, COS_C2);
    let c1 = mul_add(z, c2, COS_C1);
    let r = z * c1;
    if ix < 0x3FD3_3333 {
        // |x| < 0.3
        ONE - mul_sub(0.5, z, mul_sub(z, r, x * y))
    } else {
        let qx = if ix > 0x3FE9_0000 {
            // x > 0.78125
            0.28125
        } else {
            // x / 4, with the low word cleared
            from_words(ix - 0x0020_0000, 0)
        };
        let iz = mul_sub(0.5, z, qx);
        let a = ONE - qx;
        a - (iz - mul_sub(z, r, x * y))
    }
}

/// `__kernel_sin`: the sine on `[-pi/4, pi/4]`, for `x + y`; `iy` is 0 when `y` is zero, else 1.
fn kernel_sin(x: f64, y: f64, iy: i32) -> f64 {
    const HALF: f64 = 0.5;
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix < 0x3E40_0000 && (x as i32) == 0 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let s5 = mul_add(z, SIN_S6, SIN_S5);
    let s4 = mul_add(z, s5, SIN_S4);
    let s3 = mul_add(z, s4, SIN_S3);
    let r = mul_add(z, s3, SIN_S2);
    if iy == 0 {
        mul_add(v, mul_add(z, r, SIN_S1), x)
    } else {
        let inner = mul_sub(z, mul_sub(HALF, y, v * r), y);
        x - sub_mul(inner, v, SIN_S1)
    }
}

/// `__kernel_tan`: the tangent (`iy = 1`) or its negative inverse (`iy = -1`) on `[-pi/4, pi/4]`,
/// for `x + y`.
fn kernel_tan(x: f64, y: f64, iy: i32) -> f64 {
    let mut x = x;
    let mut y = y;
    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix < 0x3E30_0000 && (x as i32) == 0 {
        let low = low_word(x);
        if ((ix as u32 | low) | (iy + 1) as u32) == 0 {
            return TAN_ONE / x.abs();
        }
        if iy == 1 {
            return x;
        }
        let w = x + y;
        let z = set_low_word(w, 0);
        let v = y - (z - x);
        let a = -TAN_ONE / w;
        let t = set_low_word(a, 0);
        let s = mul_add(t, z, TAN_ONE);
        return mul_add(a, mul_add(t, v, s), t);
    }
    if ix >= 0x3FE5_9428 {
        if hx < 0 {
            x = -x;
            y = -y;
        }
        let z = TAN_PIO4 - x;
        let w = TAN_PIO4LO - y;
        x = z + w;
        y = 0.0;
    }
    let t = &TAN_T;
    let z = x * x;
    let w = z * z;
    let a5 = mul_add(w, t[11], t[9]);
    let a4 = mul_add(w, a5, t[7]);
    let a3 = mul_add(w, a4, t[5]);
    let a2 = mul_add(w, a3, t[3]);
    let mut r = mul_add(w, a2, t[1]);
    let b5 = mul_add(w, t[12], t[10]);
    let b4 = mul_add(w, b5, t[8]);
    let b3 = mul_add(w, b4, t[6]);
    let b2 = mul_add(w, b3, t[4]);
    let b1 = mul_add(w, b2, t[2]);
    let v = z * b1;
    let s = z * x;
    r = mul_add(z, mul_add(s, r + v, y), y);
    r = mul_add(t[0], s, r);
    let w = x + r;
    if ix >= 0x3FE5_9428 {
        let v = f64::from(iy);
        let sign = f64::from(1 - ((hx >> 30) & 2));
        return sign * sub_mul(v, 2.0, x - (w * w / (w + v) - r));
    }
    if iy == 1 {
        return w;
    }
    // iy = -1: -1/tan(x).
    let z = set_low_word(w, 0);
    let v = r - (z - x);
    let a = -1.0 / w;
    let t = set_low_word(a, 0);
    let s = mul_add(t, z, 1.0);
    mul_add(a, mul_add(t, v, s), t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        assert_eq!(sin(0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(sin(-0.0).to_bits(), (-0.0f64).to_bits());
        assert_eq!(cos(0.0), 1.0);
        assert!(sin(f64::INFINITY).is_nan());
        assert!(cos(f64::NAN).is_nan());
        assert!(tan(f64::NEG_INFINITY).is_nan());
        assert_eq!(tan(-0.0).to_bits(), (-0.0f64).to_bits());
    }
}
