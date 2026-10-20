//! JavaScript's `Math` functions, bit for bit, as the browser computes them.
//!
//! The layouts run at render time in the browser, and the geometry checks render in Chromium 151
//! (Playwright's), so the default functions are Chromium's. The parser differentials run in Node, which
//! computes some functions differently, so the [`node`] module has Node 24's (V8 13.6) functions.
//!
//! | Function | Default (Chromium 151) | [`node`] (Node 24) |
//! |---|---|---|
//! | `sin`, `cos`, `tan`, `asin`, `acos`, `log`, `log10` | correctly rounded: the value Chromium gives, which is the correctly rounded one | fdlibm (V8's `ieee754.cc`) |
//! | `atan`, `atan2` | llvm-libc (Chromium's V8 takes them from there; not correctly rounded) | fdlibm |
//! | `pow` | the platform's `pow`, with V8's special cases | the same |
//! | `hypot` | V8's two-argument formula | the same |
//!
//! Measured with the probe (`tools/port/jsmath-probe.ts`) on arm64 macOS, a million inputs per function:
//! the defaults agree with Chromium 151 on all eleven functions; [`node`] agrees with Node 24 on all
//! eleven. Node and Chromium disagree on `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log` and
//! `log10`, on 0.5% to 11% of inputs each, and agree on `pow` and `hypot`.
//!
//! **Fused multiply-adds.** V8's C++ and llvm-libc are compiled with the compiler's contraction, which
//! fuses `a * b + c` on targets with fused multiply-add instructions (every arm64 target, and x86-64 with
//! the `fma` feature). The functions that depend on it are `atan` and `atan2` (llvm-libc's
//! `LIBC_TARGET_CPU_HAS_FMA` paths), and the fdlibm functions of [`node`]. They follow [`CONTRACTS`].
//! The correctly rounded functions and `hypot` do not depend on it. The samples for the dependent functions
//! were taken on arm64 and are checked on targets that fuse; other targets are not checked.
//!
//! **Correct rounding.** The seven functions computed in `correct.rs` return the correctly rounded value by
//! Ziv's method: they compute to 256 fraction bits with an error bound of `(|v| + 1) * 2^-180`, and round
//! when both bounds round to the same double. Chromium's results are correctly rounded on every input the
//! probe checked (about 270,000 inputs, against mpmath at 300 bits; `log10` on 200,000, against Python's
//! `decimal` at 80 digits), so the two agree.
//!
//! `pow` and `hypot` are not correctly rounded in either runtime, and both runtimes agree on them. `pow` is
//! the platform's `pow`, so it is the platform's answer and only macOS was checked.
//!
//! The source of each function names the file it ports. Licences and sources are in `NOTICE.md`: the
//! llvm-libc ports (`atan`, `atan2`) are Apache-2.0 WITH LLVM-exception, and the V8 and fdlibm code is under
//! V8's BSD-style licence and fdlibm's notice (below); the rest of the crate is MIT. Nothing is `unsafe`,
//! and the crate has no dependencies.
//!
//! The fdlibm code is adapted from fdlibm (<http://www.netlib.org/fdlibm>), with this notice from
//! V8's sources:
//!
//! ```text
//! Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//! Developed at SunSoft, a Sun Microsystems, Inc. business.
//! Permission to use, copy, modify, and distribute this software is freely granted, provided that
//! this notice is preserved.
//! ```

#![forbid(unsafe_code)]

mod atan;
mod constants;
mod correct;
mod fused;
mod hypot;
mod inverse;
mod log;
mod pow;
mod rem_pio2;
mod trig;
mod words;

pub use atan::{atan, atan2};
pub use correct::{acos, asin, cos, log, log10, sin, tan};
pub use fused::CONTRACTS;
pub use hypot::hypot;
pub use pow::pow;

/// V8 13.6 (Node 24), with its fdlibm functions: what the parser differentials run against. The
/// fdlibm functions fuse multiply-adds on targets that have them (see [`CONTRACTS`]).
pub mod node {
    pub use crate::hypot::hypot;
    pub use crate::inverse::{acos, asin, atan, atan2};
    pub use crate::log::{log, log10};
    pub use crate::pow::pow;
    pub use crate::trig::{cos, sin, tan};
}
