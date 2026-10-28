//! `atan` as llvm-libc computes it, the implementation Chromium's V8 uses (llvm-libc's
//! `libc/src/__support/math/atan.h` and `atan_utils.h`, Apache-2.0 WITH LLVM-exception; see
//! `LICENSE-LLVM`). llvm-libc's result is not always the correctly rounded one, so its algorithm is
//! reproduced here step for step. The fused multiply-adds follow llvm-libc's
//! `LIBC_TARGET_CPU_HAS_FMA`, which is the target's [`CONTRACTS`](crate::CONTRACTS): on targets with
//! fused multiply-add instructions, llvm-libc uses them, and the results here depend on that.

// The literals are llvm-libc's, kept as they are so that they can be checked against its source. Some of
// them are the values of the standard constants, which the lint would have us replace.
#![allow(clippy::approx_constant)]

use crate::fused::{CONTRACTS, mul_add};

/// A double-double number: `hi + lo`, with `|lo| <= ulp(hi)/2`. Fields in llvm-libc's order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dd {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}

/// `a + b` exactly, as a double-double (llvm-libc's `exact_add`, fast two-sum).
pub(crate) fn exact_add(a: f64, b: f64) -> Dd {
    let hi = a + b;
    let t = hi - a;
    Dd { lo: b - t, hi }
}

/// `a * b` exactly, as a double-double.
pub(crate) fn exact_mult(a: f64, b: f64) -> Dd {
    let hi = a * b;
    // The error of the product is exact with a fused multiply-add; llvm-libc's split product on
    // other targets is exact too, so both paths agree.
    Dd {
        lo: mul_add(a, b, -hi),
        hi,
    }
}

/// `a / b` as a double-double (llvm-libc's `div`).
pub(crate) fn dd_div(a: Dd, b: Dd) -> Dd {
    let q = 1.0 / b.hi;
    let r_hi = a.hi * q;
    let (e_hi, e_lo) = if CONTRACTS {
        (mul_add(b.hi, -r_hi, a.hi), mul_add(b.lo, -r_hi, a.lo))
    } else {
        let bh = exact_mult(b.hi, -r_hi);
        let bl = exact_mult(b.lo, -r_hi);
        ((a.hi + bh.hi) + bh.lo, (a.lo + bl.hi) + bl.lo)
    };
    Dd {
        lo: q * (e_hi + e_lo),
        hi: r_hi,
    }
}

/// `ATAN_I[i] = (lo, hi)`: `atan(i/64)` as a double-double, from llvm-libc (`atan_utils.h`).
pub(crate) const ATAN_I: [(f64, f64); 65] = [
    (0.0, 0.0),
    (-4.913600136566304e-19, 0.015623728620476831),
    (-1.188442711587748e-18, 0.031239833430268277),
    (-1.655677442254952e-19, 0.046840712915969654),
    (-1.5490756308295046e-18, 0.06241880999595735),
    (5.804551873143357e-18, 0.0779666338315423),
    (-6.2844725995420954e-18, 0.09347678115858947),
    (6.8267122072409585e-18, 0.10894195698986579),
    (-3.1253241424539383e-18, 0.12435499454676144),
    (-2.9579864247315813e-18, 0.13970887428916365),
    (9.585415594114324e-18, 0.15499674192394097),
    (-3.541164079802125e-18, 0.1702119252854744),
    (4.180692268843079e-18, 0.18534794999569476),
    (3.1399542871844493e-18, 0.2003985538258785),
    (4.738160130078733e-19, 0.21535769969773805),
    (1.2313404529142703e-17, 0.23021958727684372),
    (1.0698755618734451e-17, 0.24497866312686414),
    (1.9238754924615304e-17, 0.2596296294082575),
    (8.261353575163773e-18, 0.2741674511196588),
    (-1.428369957377257e-17, 0.2885873618940774),
    (-1.1010827903001369e-17, 0.3028848683749714),
    (-1.893928924292642e-17, 0.31705575320914703),
    (-7.952610375793799e-18, 0.3310960767041321),
    (-2.2938804755578304e-17, 0.34500217720710513),
    (-2.4623815582638635e-17, 0.35877067027057225),
    (1.9612311504845653e-17, 0.3723984466767542),
    (2.378822732491941e-17, 0.38588266939807375),
    (2.246598105617042e-17, 0.39922076957525254),
    (-1.587652227770689e-17, 0.4124104415973873),
    (2.3315530741892885e-17, 0.42544963737004227),
    (-2.494277030626541e-17, 0.43833655985795783),
    (-2.2703795229420475e-17, 0.4510696559885235),
    (2.2698777452961687e-17, 0.4636476090008061),
    (1.4654487332256713e-17, 0.4760693303227612),
    (-1.1373236189329585e-17, 0.48833395105640554),
    (-4.7181675085518756e-17, 0.5004408131472942),
    (-2.5462781472855804e-17, 0.5123894603107377),
    (5.520094119641666e-18, 0.5241796287829132),
    (-4.0637956834825575e-18, 0.5358112379604637),
    (4.923709671396255e-17, 0.5472843809874369),
    (-5.4556305485916264e-18, 0.5585993153435624),
    (1.2255062085054184e-17, 0.5697564534829784),
    (-1.441464378193067e-17, 0.5807563535676704),
    (4.920495453686772e-17, 0.5915997103351114),
    (2.950430737228402e-17, 0.6022873461349642),
    (-3.1552061848586226e-17, 0.6128202021652414),
    (2.672403885140095e-17, 0.6231993299340659),
    (-2.7290767436015276e-17, 0.6334258829691446),
    (1.5834785051444286e-17, 0.6435011087932844),
    (3.5800634857340095e-17, 0.6534263411807619),
    (-3.076054864429649e-17, 0.6632029927060933),
    (-1.899315009714705e-17, 0.6728325475937632),
    (6.943223671560008e-18, 0.6823165548747481),
    (-8.117151192285796e-18, 0.6916566218531999),
    (-1.987626234335816e-17, 0.7008544078844502),
    (-4.597166450584887e-17, 0.7099116184635249),
    (-2.1478388444456983e-17, 0.7188299996216245),
    (2.569325697391839e-18, 0.7276113326265107),
    (3.473937648299457e-17, 0.7362574289814281),
    (3.708315849135547e-17, 0.7447701257160751),
    (-2.4256934659182068e-17, 0.7531512809621944),
    (9.850030332752822e-18, 0.7614027698055784),
    (-3.704991905602721e-17, 0.7695264804056583),
    (-2.6676490951944502e-17, 0.7775243103733478),
    (3.061616997868383e-17, 0.7853981633974483),
];

/// `atan(x)` for `|x| < 1` in `x = (x_r)` form: the polynomial of llvm-libc's `atan_eval`.
pub(crate) fn atan_eval(x: Dd) -> Dd {
    let x_hi_sq = x.hi * x.hi;
    let c0 = mul_add(x_hi_sq, 0.2, -0.3333333333333333);
    let c1 = mul_add(x_hi_sq, 0.1111111111111111, -0.14285714285714285);
    let x_hi_3 = x_hi_sq * x.hi;
    let x_hi_4 = x_hi_sq * x_hi_sq;
    let d0 = mul_add(x_hi_4, c1, c0);
    let d1 = mul_add(x_hi_4 - x_hi_sq, x.lo, x.lo);
    Dd {
        lo: mul_add(x_hi_3, d0, d1),
        hi: x.hi,
    }
}

const PI_OVER_2: Dd = Dd {
    lo: 6.123233995736766e-17,
    hi: 1.5707963267948966,
};
const MPI_OVER_2: Dd = Dd {
    lo: -6.123233995736766e-17,
    hi: -1.5707963267948966,
};

/// `atan(x)`, as llvm-libc computes it.
#[must_use]
pub fn atan(x: f64) -> f64 {
    let bits = x.to_bits();
    let x_sign = bits >> 63 == 1;
    let x_abs = f64::from_bits(bits & !(1u64 << 63));
    let x_exp = ((x_abs.to_bits() >> 52) as i32) - 1023;
    let sign = |negative: bool| if negative { -1.0 } else { 1.0 };
    if x.is_nan() {
        return f64::NAN;
    }
    if x_exp < 0 {
        if x_exp < -26 {
            if x == 0.0 {
                return x;
            }
            return mul_add(-5.551115123125783e-17, x, x);
        }
        let k = (64.0 * x_abs).round_ties_even();
        let idx = k as usize;
        let k = k * 0.015625;
        let num = Dd {
            lo: 0.0,
            hi: x_abs - k,
        };
        let den_hi = mul_add(x_abs, k, 1.0);
        let prod = exact_mult(x_abs, k);
        let den = Dd {
            lo: ((1.0 - den_hi) + prod.hi) + prod.lo,
            hi: den_hi,
        };
        let x_r = dd_div(num, den);
        let p = atan_eval(x_r);
        let c0 = exact_add(ATAN_I[idx].1, p.hi);
        let c1 = c0.lo + (ATAN_I[idx].0 + p.lo);
        return sign(x_sign) * (c0.hi + c1);
    }
    if x_exp >= 53 {
        // |x| >= 2^53, including infinities
        return mul_add(sign(x_sign), PI_OVER_2.hi, sign(x_sign) * PI_OVER_2.lo);
    }
    let y = 1.0 / x_abs;
    let k = (64.0 * y).round_ties_even();
    let idx = k as usize;
    let k = k * 0.015625;
    let den = exact_add(x_abs, k);
    let num_hi = mul_add(-x_abs, k, 1.0);
    let prod = exact_mult(x_abs, k);
    let num = Dd {
        lo: ((1.0 - num_hi) - prod.hi) - prod.lo,
        hi: num_hi,
    };
    let x_r = dd_div(num, den);
    let p = atan_eval(x_r);
    let c0 = exact_add(MPI_OVER_2.hi, ATAN_I[idx].1);
    let c1 = exact_add(c0.hi, p.hi);
    let c2 = c1.lo + (c0.lo + p.lo) + (ATAN_I[idx].0 + MPI_OVER_2.lo);
    sign(!x_sign) * (c1.hi + c2)
}

/// The double-double sum of two double-doubles (llvm-libc's `add`, fast two-sum).
pub(crate) fn dd_add(a: Dd, b: Dd) -> Dd {
    let r = exact_add(a.hi, b.hi);
    let lo = a.lo + b.lo;
    let r_lo = r.lo + lo;
    exact_add(r.hi, r_lo)
}

const ZERO: Dd = Dd { lo: 0.0, hi: 0.0 };
const MZERO: Dd = Dd { lo: -0.0, hi: -0.0 };
const PI: Dd = Dd {
    lo: 1.2246467991473532e-16,
    hi: 3.141592653589793,
};
const MPI: Dd = Dd {
    lo: -1.2246467991473532e-16,
    hi: -3.141592653589793,
};
const PI_OVER_4: Dd = Dd {
    lo: 3.061616997868383e-17,
    hi: 0.7853981633974483,
};
const THREE_PI_OVER_4: Dd = Dd {
    lo: 9.184850993605148e-17,
    hi: 2.356194490192345,
};

/// `atan2(y, x)`, as llvm-libc computes it.
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    let sign = |negative: bool| if negative { -1.0 } else { 1.0 };
    // CONST_ADJ[x_sign][y_sign][recip]
    let const_adj = [
        [[ZERO, MPI_OVER_2], [MZERO, MPI_OVER_2]],
        [[MPI, PI_OVER_2], [MPI, PI_OVER_2]],
    ];
    let x_sign = x.to_bits() >> 63 == 1;
    let y_sign = y.to_bits() >> 63 == 1;
    let x_abs = x.to_bits() & !(1u64 << 63);
    let y_abs = y.to_bits() & !(1u64 << 63);
    let recip = x_abs < y_abs;
    let mut min_abs = if recip { x_abs } else { y_abs };
    let mut max_abs = if !recip { x_abs } else { y_abs };
    let mut min_exp = (min_abs >> 52) as u32;
    let mut max_exp = (max_abs >> 52) as u32;
    let mut num = f64::from_bits(min_abs);
    let mut den = f64::from_bits(max_abs);
    if max_exp > 0x7FF - 128 || min_exp < 128 {
        if x.is_nan() || y.is_nan() {
            return f64::NAN;
        }
        let x_except = if x == 0.0 {
            0
        } else if f64::from_bits(x_abs).is_infinite() {
            2
        } else {
            1
        };
        let y_except = if y == 0.0 {
            0
        } else if f64::from_bits(y_abs).is_infinite() {
            2
        } else {
            1
        };
        // EXCEPTS[y_except][x_except][x_sign]
        const EXCEPTS: [[[Dd; 2]; 3]; 3] = [
            [[ZERO, PI], [ZERO, PI], [ZERO, PI]],
            [[PI_OVER_2, PI_OVER_2], [ZERO, ZERO], [ZERO, PI]],
            [
                [PI_OVER_2, PI_OVER_2],
                [PI_OVER_2, PI_OVER_2],
                [PI_OVER_4, THREE_PI_OVER_4],
            ],
        ];
        if x_except != 1 || y_except != 1 {
            let r = EXCEPTS[y_except][x_except][usize::from(x_sign)];
            return mul_add(sign(y_sign), r.hi, sign(y_sign) * r.lo);
        }
        let scale_up = min_exp < 128;
        let scale_down = max_exp > 0x7FF - 128;
        if scale_up {
            num *= 2f64.powi(64);
            if !scale_down {
                den *= 2f64.powi(64);
            }
        } else if scale_down {
            den *= 2f64.powi(-64);
            if !scale_up {
                num *= 2f64.powi(-64);
            }
        }
        min_abs = num.to_bits();
        max_abs = den.to_bits();
        min_exp = (min_abs >> 52) as u32;
        max_exp = (max_abs >> 52) as u32;
    }
    let final_sign = sign((x_sign != y_sign) != recip);
    let const_term = const_adj[usize::from(x_sign)][usize::from(y_sign)][usize::from(recip)];
    let exp_diff = max_exp - min_exp;
    if exp_diff > 54 {
        return mul_add(
            final_sign,
            const_term.hi,
            final_sign * (const_term.lo + num / den),
        );
    }
    let k = (64.0 * num / den).round_ties_even();
    let idx = k as usize;
    let k = k * 2f64.powi(-6);
    let num_k = exact_mult(num, k);
    let den_k = exact_mult(den, k);
    let num_dd = exact_add(num - den_k.hi, -den_k.lo);
    let mut den_dd = exact_add(den, num_k.hi);
    den_dd.lo += num_k.lo;
    let q = dd_div(num_dd, den_dd);
    let p = atan_eval(q);
    let mut r = dd_add(const_term, dd_add(atan_table(idx), p));
    r.hi *= final_sign;
    r.lo *= final_sign;
    r.hi + r.lo
}

fn atan_table(idx: usize) -> Dd {
    let (lo, hi) = ATAN_I[idx];
    Dd { lo, hi }
}
