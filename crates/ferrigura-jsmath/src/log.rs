//! The natural logarithm as fdlibm computes it in V8 (`ieee754::log`, fdlibm's `e_log.c`). The
//! multiply-adds follow V8's compiler (see [`crate::fused`]).

use crate::constants::{
    LOG_LG1, LOG_LG2, LOG_LG3, LOG_LG4, LOG_LG5, LOG_LG6, LOG_LG7, LOG_LN2_HI, LOG_LN2_LO,
};
use crate::fused::{mul_add, mul_sub, sub_mul};
use crate::words::{high_word, low_word, set_high_word};

/// `ln(x)` (V8's `ieee754::log`). Negative numbers give `NaN`, zero gives `-Infinity`.
#[must_use]
pub fn log(x: f64) -> f64 {
    let mut x = x;
    let mut hx = high_word(x);
    let lx = low_word(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        // x < 2^-1022: zero, negative, or subnormal.
        if ((hx & 0x7FFF_FFFF) as u32 | lx) == 0 {
            return f64::NEG_INFINITY;
        }
        if hx < 0 {
            return f64::NAN;
        }
        // Scale subnormals up by 2^54.
        k -= 54;
        x *= f64::from_bits(0x4350_0000_0000_0000);
        hx = high_word(x);
    }
    if hx >= 0x7FF0_0000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000F_FFFF;
    let i = (hx + 0x0009_5F64) & 0x0010_0000;
    // x with its exponent set so that x is in [sqrt(1/2), sqrt(2))
    x = set_high_word(x, hx | (i ^ 0x3FF0_0000));
    k += i >> 20;
    let f = x - 1.0;
    let dk = f64::from(k);
    if (0x000F_FFFF & (2 + hx)) < 3 {
        // |f| < 2^-20
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            return mul_add(dk, LOG_LN2_HI, dk * LOG_LN2_LO);
        }
        let r = f * f * sub_mul(0.5, 0.333_333_333_333_333_3, f);
        if k == 0 {
            return f - r;
        }
        return mul_sub(dk, LOG_LN2_HI, sub_mul(r, dk, LOG_LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let z = s * s;
    let mut i = hx - 0x0006_147A;
    let w = z * z;
    let j = 0x0006_B851 - hx;
    let t1 = w * mul_add(w, mul_add(w, LOG_LG6, LOG_LG4), LOG_LG2);
    let t2 = z * mul_add(
        w,
        mul_add(w, mul_add(w, LOG_LG7, LOG_LG5), LOG_LG3),
        LOG_LG1,
    );
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - sub_mul(hfsq, s, hfsq + r)
        } else {
            mul_sub(
                dk,
                LOG_LN2_HI,
                hfsq - mul_add(s, hfsq + r, dk * LOG_LN2_LO) - f,
            )
        }
    } else if k == 0 {
        sub_mul(f, s, f - r)
    } else {
        mul_sub(dk, LOG_LN2_HI, mul_sub(s, f - r, dk * LOG_LN2_LO) - f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        assert_eq!(log(1.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(log(0.0), f64::NEG_INFINITY);
        assert_eq!(log(-0.0), f64::NEG_INFINITY);
        assert!(log(-1.0).is_nan());
        assert_eq!(log(f64::INFINITY), f64::INFINITY);
        assert!(log(f64::NAN).is_nan());
        assert!(log(f64::MIN_POSITIVE / 4.0).is_finite());
    }
}
