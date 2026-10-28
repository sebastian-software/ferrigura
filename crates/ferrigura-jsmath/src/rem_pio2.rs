//! Argument reduction modulo pi/2, as fdlibm does it (`__ieee754_rem_pio2` and `__kernel_rem_pio2`).
//!
//! The port follows V8's `src/base/ieee754.cc`. V8 calls the kernel only with `prec = 2`, the
//! double-precision result, so only that branch is ported.

use crate::constants::{
    NPIO2_HW, PIO2_TERMS, REM_INVPIO2, REM_PIO2_1, REM_PIO2_1T, REM_PIO2_2, REM_PIO2_2T,
    REM_PIO2_3, REM_PIO2_3T, TWO_OVER_PI, TWO24, TWON24,
};
use crate::fused::{mul_add, mul_sub, sub_mul};
use crate::words::{from_words, high_word, low_word, scalbn, set_high_word};

/// Reduces `x` modulo pi/2. Returns the quadrant `n` and stores the remainder in `y`, so that
/// `x = n * pi/2 + y[0] + y[1]` with `y[0] + y[1]` in about `[-pi/4, pi/4]`. The low two bits of
/// `n` select the function in `sin`, `cos` and `tan`.
pub(crate) fn rem_pio2(x: f64, y: &mut [f64; 2]) -> i32 {
    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        // |x| ~<= pi/4, no reduction needed.
        y[0] = x;
        y[1] = 0.0;
        return 0;
    }
    if ix < 0x4002_D97C {
        // |x| < 3pi/4, special case with n = +-1.
        if hx > 0 {
            let mut z = x - REM_PIO2_1;
            if ix != 0x3FF9_21FB {
                // 33+53 bit pi is good enough.
                y[0] = z - REM_PIO2_1T;
                y[1] = (z - y[0]) - REM_PIO2_1T;
            } else {
                // Near pi/2, use 33+33+53 bit pi.
                z -= REM_PIO2_2;
                y[0] = z - REM_PIO2_2T;
                y[1] = (z - y[0]) - REM_PIO2_2T;
            }
            return 1;
        }
        let mut z = x + REM_PIO2_1;
        if ix != 0x3FF9_21FB {
            y[0] = z + REM_PIO2_1T;
            y[1] = (z - y[0]) + REM_PIO2_1T;
        } else {
            z += REM_PIO2_2;
            y[0] = z + REM_PIO2_2T;
            y[1] = (z - y[0]) + REM_PIO2_2T;
        }
        return -1;
    }
    if ix <= 0x4139_21FB {
        // |x| ~<= 2^19 * (pi/2), medium size.
        let t = x.abs();
        let n = mul_add(t, REM_INVPIO2, 0.5) as i32;
        let fnn = f64::from(n);
        let mut r = sub_mul(t, fnn, REM_PIO2_1);
        let mut w = fnn * REM_PIO2_1T;
        if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
            // The first round is good to 85 bits, and there is no cancellation.
            y[0] = r - w;
        } else {
            let j = ix >> 20;
            y[0] = r - w;
            let high = ((high_word(y[0]) as u32) >> 20) & 0x7FF;
            let mut i = j - high as i32;
            if i > 16 {
                // A second iteration is needed, good to 118 bits.
                let t = r;
                w = fnn * REM_PIO2_2;
                r = t - w;
                w = mul_sub(fnn, REM_PIO2_2T, (t - r) - w);
                y[0] = r - w;
                let high = ((high_word(y[0]) as u32) >> 20) & 0x7FF;
                i = j - high as i32;
                if i > 49 {
                    // A third iteration is needed, 151 bits.
                    let t = r;
                    w = fnn * REM_PIO2_3;
                    r = t - w;
                    w = mul_sub(fnn, REM_PIO2_3T, (t - r) - w);
                    y[0] = r - w;
                }
            }
        }
        y[1] = (r - y[0]) - w;
        return if hx < 0 {
            y[0] = -y[0];
            y[1] = -y[1];
            -n
        } else {
            n
        };
    }
    // All other (large) arguments.
    if ix >= 0x7FF0_0000 {
        // x is infinite or NaN.
        y[0] = f64::NAN;
        y[1] = f64::NAN;
        return 0;
    }
    // Set z = scalbn(|x|, ilogb(x) - 23), built from the words as the C code does.
    let e0 = (ix >> 20) - 1046;
    let mut z = set_high_word(
        from_words(0, low_word(x)),
        ix.wrapping_sub(e0.wrapping_mul(1 << 20)),
    );
    let mut tx = [0.0f64; 3];
    for slot in tx.iter_mut().take(2) {
        *slot = f64::from(z as i32);
        z = (z - *slot) * TWO24;
    }
    tx[2] = z;
    let mut nx = 3;
    while tx[nx - 1] == 0.0 {
        nx -= 1;
    }
    let n = kernel_rem_pio2(&tx[..nx], y, e0, &TWO_OVER_PI);
    if hx < 0 {
        y[0] = -y[0];
        y[1] = -y[1];
        -n
    } else {
        n
    }
}

/// `__kernel_rem_pio2` with `prec = 2`: the reduction of `x`, a number split into 24-bit pieces,
/// modulo pi/2. Returns the quadrant in its low three bits.
fn kernel_rem_pio2(x: &[f64], y: &mut [f64; 2], e0: i32, ipio2: &[i32]) -> i32 {
    // init_jk[prec] for prec = 2.
    let jk: i32 = 4;
    let jp = jk;
    let jx = x.len() as i32 - 1;
    let mut jv = (e0 - 3) / 24;
    if jv < 0 {
        jv = 0;
    }
    let mut q0 = e0 - 24 * (jv + 1);

    let mut f = [0.0f64; 20];
    let m = jx + jk;
    for i in 0..=m {
        let j = jv - jx + i;
        f[i as usize] = if j < 0 {
            0.0
        } else {
            f64::from(ipio2[j as usize])
        };
    }

    let mut q = [0.0f64; 20];
    for i in 0..=jk {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw = mul_add(x[j as usize], f[(jx + i - j) as usize], fw);
        }
        q[i as usize] = fw;
    }

    let mut iq = [0i32; 20];
    let mut jz = jk;
    let mut z;
    let mut n;
    let mut ih;
    loop {
        // The integer parts of the terms, from the most significant down.
        z = q[jz as usize];
        let mut j = jz;
        let mut i = 0usize;
        while j > 0 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[i] = sub_mul(z, TWO24, fw) as i32;
            z = q[(j - 1) as usize] + fw;
            i += 1;
            j -= 1;
        }

        z = scalbn(z, q0);
        z = sub_mul(z, 8.0, (z * 0.125).floor());
        n = z as i32;
        z -= f64::from(n);
        ih = 0;
        if q0 > 0 {
            let i = iq[(jz - 1) as usize] >> (24 - q0);
            n += i;
            iq[(jz - 1) as usize] -= i << (24 - q0);
            ih = iq[(jz - 1) as usize] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[(jz - 1) as usize] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            n += 1;
            let mut carry = 0;
            for slot in iq.iter_mut().take(jz as usize) {
                let j = *slot;
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        *slot = 0x100_0000 - j;
                    }
                } else {
                    *slot = 0xFF_FFFF - j;
                }
            }
            if q0 > 0 {
                match q0 {
                    1 => iq[(jz - 1) as usize] &= 0x7F_FFFF,
                    2 => iq[(jz - 1) as usize] &= 0x3F_FFFF,
                    _ => {}
                }
            }
            if ih == 2 {
                z = 1.0 - z;
                if carry != 0 {
                    z -= scalbn(1.0, q0);
                }
            }
        }

        if z == 0.0 {
            let mut j = 0;
            let mut i = jz - 1;
            while i >= jk {
                j |= iq[i as usize];
                i -= 1;
            }
            if j == 0 {
                // The remaining terms of 2/pi are all zero: take more of them, then recompute.
                let mut k = 1;
                while jk >= k && iq[(jk - k) as usize] == 0 {
                    k += 1;
                }
                for i in (jz + 1)..=(jz + k) {
                    f[(jx + i) as usize] = f64::from(ipio2[(jv + i) as usize]);
                    let mut fw = 0.0;
                    for j in 0..=jx {
                        fw = mul_add(x[j as usize], f[(jx + i - j) as usize], fw);
                    }
                    q[i as usize] = fw;
                }
                jz += k;
                continue;
            }
        }
        break;
    }

    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[jz as usize] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        z = scalbn(z, -q0);
        if z >= TWO24 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[jz as usize] = sub_mul(z, TWO24, fw) as i32;
            jz += 1;
            q0 += 24;
            iq[jz as usize] = fw as i32;
        } else {
            iq[jz as usize] = z as i32;
        }
    }

    let mut fw = scalbn(1.0, q0);
    for i in (0..=jz).rev() {
        q[i as usize] = fw * f64::from(iq[i as usize]);
        fw *= TWON24;
    }

    let mut fq = [0.0f64; 20];
    for i in (0..=jz).rev() {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= jp && k <= jz - i {
            fw = mul_add(PIO2_TERMS[k as usize], q[(i + k) as usize], fw);
            k += 1;
        }
        fq[(jz - i) as usize] = fw;
    }

    // prec = 2: the sum of the terms, then the tail.
    let mut fw = 0.0;
    for i in (0..=jz).rev() {
        fw += fq[i as usize];
    }
    y[0] = if ih == 0 { fw } else { -fw };
    fw = fq[0] - fw;
    for i in 1..=jz {
        fw += fq[i as usize];
    }
    y[1] = if ih == 0 { fw } else { -fw };
    n & 7
}
