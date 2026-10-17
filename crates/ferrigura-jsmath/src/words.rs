//! The two 32-bit halves of a double, as fdlibm reads and writes them.
//!
//! V8's `EXTRACT_WORDS`, `GET_HIGH_WORD`, `GET_LOW_WORD`, `INSERT_WORDS`, `SET_HIGH_WORD` and
//! `SET_LOW_WORD` macros, and `scalbn` from the C library, which fdlibm calls.

/// The high word of `x` as a signed integer (`GET_HIGH_WORD`).
#[inline]
pub(crate) fn high_word(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}

/// The low word of `x` (`GET_LOW_WORD`).
#[inline]
pub(crate) fn low_word(x: f64) -> u32 {
    x.to_bits() as u32
}

/// The double with the given words (`INSERT_WORDS`).
#[inline]
pub(crate) fn from_words(high: i32, low: u32) -> f64 {
    f64::from_bits(((high as u32 as u64) << 32) | u64::from(low))
}

/// `x` with its high word replaced (`SET_HIGH_WORD`).
#[inline]
pub(crate) fn set_high_word(x: f64, high: i32) -> f64 {
    from_words(high, low_word(x))
}

/// `x` with its low word replaced (`SET_LOW_WORD`).
#[inline]
pub(crate) fn set_low_word(x: f64, low: u32) -> f64 {
    from_words(high_word(x), low)
}

/// `x * 2^n`, rounded once (`scalbn`).
///
/// This is the algorithm of musl's `scalbn`, which avoids double rounding in the subnormal range by
/// scaling in steps. fdlibm only calls it with results in the normal range, where it is exact.
pub(crate) fn scalbn(x: f64, n: i32) -> f64 {
    const TWO_1023: f64 = f64::from_bits(0x7FE0_0000_0000_0000);
    const TWO_M1022_TIMES_TWO_53: f64 = f64::from_bits(0x0360_0000_0000_0000);
    let mut y = x;
    let mut n = n;
    if n > 1023 {
        y *= TWO_1023;
        n -= 1023;
        if n > 1023 {
            y *= TWO_1023;
            n -= 1023;
            if n > 1023 {
                n = 1023;
            }
        }
    } else if n < -1022 {
        y *= TWO_M1022_TIMES_TWO_53;
        n += 1022 - 53;
        if n < -1022 {
            y *= TWO_M1022_TIMES_TWO_53;
            n += 1022 - 53;
            if n < -1022 {
                n = -1022;
            }
        }
    }
    let scale = f64::from_bits(((0x3FF + n) as u64) << 52);
    y * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_round_trip() {
        for x in [0.0, -0.0, 1.0, -2.5, f64::MIN_POSITIVE, f64::MAX, 5e-324] {
            let rebuilt = from_words(high_word(x), low_word(x));
            assert_eq!(rebuilt.to_bits(), x.to_bits());
        }
        assert_eq!(high_word(-1.0), 0xBFF0_0000_u32 as i32);
        assert_eq!(set_low_word(1.0, 7).to_bits(), 1.0f64.to_bits() | 7);
        assert_eq!(set_high_word(1.0, 0x4000_0000).to_bits() >> 32, 0x4000_0000);
    }

    #[test]
    fn scalbn_is_exact_in_the_normal_range() {
        assert_eq!(scalbn(1.5, 10), 1536.0);
        assert_eq!(scalbn(1.5, -10), 1.5 / 1024.0);
        assert_eq!(scalbn(3.0, 0), 3.0);
        assert_eq!(scalbn(1.0, 1100), f64::INFINITY);
        assert_eq!(scalbn(1.0, -1100), 0.0);
    }
}
