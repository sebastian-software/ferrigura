# Notice: sources and licences of ferrigura-jsmath

The crate is MIT-licensed (see `LICENSE`). Parts of it are ported from other sources, and each part
keeps the licence of its source. `LICENSE-LLVM` and `LICENSE-V8` carry the licence texts.

## The default functions (Chromium 151)

| Function                                   | Where                                                                        | Source                                                                                                                                                 | Licence                                         | Depends on the target's fused multiply-adds?                                                              |
| ------------------------------------------ | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `sin`, `cos`, `tan`, `asin`, `acos`, `log` | `src/correct.rs`, constants in `src/constants.rs` (`CR_*`)                   | Written for this crate: a correctly rounded evaluator (Ziv's method, 384-bit fixed point). No code is copied. The constants were computed with mpmath. | MIT (this crate)                                | No. The fixed-point arithmetic is integer arithmetic, and the results are the correctly rounded values.   |
| `atan`, `atan2`                            | `src/atan.rs` (the table `ATAN_I`, `atan_eval`, `dd_div`, the special cases) | llvm-libc: `libc/src/__support/math/atan.h`, `atan2.h`, `atan_utils.h`, and the `FPUtil` helpers they use. Reproduced step by step.                    | Apache-2.0 WITH LLVM-exception (`LICENSE-LLVM`) | Yes. llvm-libc uses fused multiply-adds where its `LIBC_TARGET_CPU_HAS_FMA` is set, which is `CONTRACTS`. |
| `pow`                                      | `src/pow.rs`                                                                 | V8's `src/numbers/ieee754.cc` (the special cases around `std::pow`), and the platform's `pow`                                                          | BSD-style (`LICENSE-V8`)                        | The platform's `pow` decides it.                                                                          |
| `hypot`                                    | `src/hypot.rs`                                                               | V8's `src/builtins/math.tq` (`FastMathHypot`, the two-argument form)                                                                                   | BSD-style (`LICENSE-V8`)                        | No.                                                                                                       |

## The `node` module (V8 13.6, Node 24)

| Function                                                    | Where                                                                                                                                      | Source                                                                                                                   | Licence                                        | Depends on the target's fused multiply-adds?                                                        |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log` | `src/trig.rs`, `src/inverse.rs`, `src/log.rs`, `src/rem_pio2.rs`, constants in `src/constants.rs` (fdlibm), `src/fused.rs`, `src/words.rs` | V8's `src/base/ieee754.cc` (V8 13.6.233.17), adapted from fdlibm (Sun Microsystems, 1993; the notice is in `src/lib.rs`) | BSD-style (`LICENSE-V8`) and the fdlibm notice | Yes: V8's compiler fuses them on targets that have fused multiply-adds, and `CONTRACTS` follows it. |
| `pow`, `hypot`                                              | `src/pow.rs`, `src/hypot.rs`                                                                                                               | As above                                                                                                                 | BSD-style (`LICENSE-V8`)                       | As above                                                                                            |

`src/words.rs`, `scalbn`, follows the algorithm of musl's `scalbn` (MIT, Rich Felker and the musl
contributors). It is written out again here, not copied.

## Test data

`tests/data/chromium` holds Chromium 151's results and `tests/data/node` Node 24's, for samples of
the probe's inputs (`tools/port/jsmath-probe.ts`). The data are outputs of the two runtimes, not
copies of any source.
