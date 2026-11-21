# ferrigura-jsmath

`ferrigura-jsmath` gives JavaScript's `Math` functions as V8 computes them, bit for bit: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log`, `log10`, `pow` and `hypot`. The default functions match Chromium 151, and the `node` module matches V8 13.6 in Node 24. The layouts of ferrigura take their trigonometry and logarithms from here, so they do not depend on the standard library of the build. `pow` is decided by the platform's `pow`, as it is in Chromium.

This [Ferrigura](https://github.com/sebastian-software/ferrigura) implementation crate exposes an experimental API. The [`ferrigura` facade](https://crates.io/crates/ferrigura) provides the [supported Rust API](https://github.com/sebastian-software/ferrigura/blob/main/docs/decisions/0022-stable-public-api.md).

## License

The crate is MIT-licensed. Its ported parts keep the licenses of their sources: V8's BSD-style license (`LICENSE-V8`), the Apache-2.0 license with the LLVM exception of llvm-libc (`LICENSE-LLVM`), fdlibm's notice, and the MIT-licensed CORE-MATH `acos` fast path (`LICENSE-CORE-MATH`). `NOTICE.md` lists each part and its license.
