# ferrigura-jsmath

`ferrigura-jsmath` gives JavaScript's `Math` functions as V8 computes them, bit for bit: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log`, `log10`, `pow` and `hypot`. The default functions match Chromium 151, and the `node` module matches V8 13.6 in Node 24. The layouts of ferrigura take their trigonometry and logarithms from here, so they do not depend on the standard library of the build. `pow` is decided by the platform's `pow`, as it is in Chromium.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

The crate is not published yet.

## Licence

The crate is MIT-licensed. Its ported parts keep the licences of their sources: V8's BSD-style licence (`LICENSE-V8`), the Apache-2.0 licence with the LLVM exception of llvm-libc (`LICENSE-LLVM`), and fdlibm's notice. `NOTICE.md` lists each part and its licence.
