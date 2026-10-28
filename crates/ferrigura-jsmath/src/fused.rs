//! The fused multiply-adds that V8's C++ compiler makes in fdlibm.
//!
//! clang and GCC contract a multiplication that is an operand of an addition or subtraction in the
//! same expression into one fused multiply-add, which rounds once. They do so when the target has
//! fused multiply-add instructions, which is every arm64 target, and x86-64 only when built with the
//! `fma` feature: the x86-64 baseline has none, so V8 does not contract there. Measured on arm64, V8
//! 13.6's fdlibm code compiled with clang's default contraction agrees with Node's `Math` on every
//! probed input, and compiled without contraction it differs in the last bit on up to 0.6% of them.
//!
//! The operand order follows clang: when both operands of `+` or `-` are products, the left one is
//! fused. The helpers take the operands of the C expression, and compute the expression as V8's
//! compiler does when [`CONTRACTS`] is true, and as written otherwise.

/// Whether the target fuses multiply-adds, as V8's compiler does for it. When it does, the fdlibm
/// functions compute with fused operations, and their results are those of V8 on the same target.
pub const CONTRACTS: bool = cfg!(any(target_arch = "aarch64", target_feature = "fma"));

/// `a * b + c`.
#[inline]
pub(crate) fn mul_add(a: f64, b: f64, c: f64) -> f64 {
    if CONTRACTS {
        a.mul_add(b, c)
    } else {
        a * b + c
    }
}

/// `a * b - c`.
#[inline]
pub(crate) fn mul_sub(a: f64, b: f64, c: f64) -> f64 {
    if CONTRACTS {
        a.mul_add(b, -c)
    } else {
        a * b - c
    }
}

/// `c - a * b`.
#[inline]
pub(crate) fn sub_mul(c: f64, a: f64, b: f64) -> f64 {
    if CONTRACTS {
        (-a).mul_add(b, c)
    } else {
        c - a * b
    }
}
