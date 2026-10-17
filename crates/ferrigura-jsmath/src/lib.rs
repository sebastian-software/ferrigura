//! JavaScript's `Math` functions as V8 computes them, bit for bit.
//!
//! The P2 layouts (cose-bilkent and fcose) compute with `Math.sin`, `Math.cos`, `Math.log` and
//! `Math.pow`, and the ishikawa renderer computes angles with `Math.sin` and `Math.cos`. Rust's `f64`
//! methods call the platform's libm, which differs from V8 in the last bit for a few percent of
//! inputs, and a layout amplifies one bit into pixels. The functions here are V8's own.
//!
//! The reference is **Node 24 (V8 13.6.233.17)**, which the probe in `tools/port/jsmath-probe.ts`
//! runs. What each function is, and what depends on the platform:
//!
//! | Function | V8's implementation | Platform dependence |
//! |---|---|---|
//! | [`sin`], [`cos`], [`tan`] | fdlibm (`ieee754::sin` and friends, `src/base/ieee754.cc`) | fused multiply-adds, see below |
//! | [`atan`], [`atan2`], [`asin`], [`acos`] | fdlibm (`ieee754::atan` and so on) | fused multiply-adds |
//! | [`log`] | fdlibm (`ieee754::log`) | fused multiply-adds |
//! | [`hypot`] | Torque, the two-argument fast path (`src/builtins/math.tq`) | none |
//! | [`pow`] | `std::pow` by default (`--use-std-math-pow`), with V8's special cases | the platform's `pow` |
//!
//! **Fused multiply-adds.** V8's C++ compiler contracts `a * b + c` into one fused operation on
//! targets that have them. Node on arm64 is built that way, so the fdlibm functions here fuse on
//! aarch64 and on x86-64 built with the `fma` feature, as [`CONTRACTS`] describes. On the other targets
//! (x86-64 baseline) V8 does not fuse, and its fdlibm results differ from these in the last bit on
//! up to about 0.6% of inputs. Those V8 builds are not checked here: the probe would need to run on
//! them to produce samples.
//!
//! **Chromium is a different reference.** The geometry checks render in Chromium, and Chromium 151's
//! `Math.sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `log` and `atan2` differ from Node's on 0.5% to
//! 11% of inputs (measured on arm64 macOS). Current V8 takes those functions from `llvm-libc`, not
//! from fdlibm (`third_party/llvm-libc` in V8's source), which is the likely cause; `hypot` and `pow`
//! agree in both. This crate does not match Chromium.
//!
//! The source of each function names the V8 file it ports. The port keeps the expression trees of
//! the C code, so the rounding of every intermediate result is the same. Nothing here is `unsafe`,
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

mod constants;
mod fused;
mod hypot;
mod inverse;
mod log;
mod pow;
mod rem_pio2;
mod trig;
mod words;

pub use fused::CONTRACTS;
pub use hypot::hypot;
pub use inverse::{acos, asin, atan, atan2};
pub use log::log;
pub use pow::pow;
pub use trig::{cos, sin, tan};
