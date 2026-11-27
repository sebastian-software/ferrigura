//! Fast rounding test adapted from CORE-MATH cr_acos, commit 7457819.
//! The ambiguous interval falls back to the crate's fixed-point evaluator.
//! Copyright (c) 2024-2025 Alexei Sibidanov; see LICENSE-CORE-MATH.
//!
//! Explicit `mul_add` calls correspond to the upstream FMA operations. Other
//! expressions remain unfused, as required by the error bounds.

const COEFF: [[f64; 8]; 33] = [
    [
        f64::from_bits(0x3ff0000000000000),
        f64::from_bits(0x0000000000000000),
        f64::from_bits(0x3fc5555555555555),
        f64::from_bits(0x3fb33333333333e4),
        f64::from_bits(0x3fa6db6db6d31f82),
        f64::from_bits(0x3f9f1c71f6889397),
        f64::from_bits(0x3f96e874b7045b46),
        f64::from_bits(0x3f91f753132271e2),
    ],
    [
        f64::from_bits(0x3ff0055a27e0d033),
        f64::from_bits(0xbc9d9ba10494c062),
        f64::from_bits(0x3fc57c00cb5d6c4d),
        f64::from_bits(0x3fb37881f5649a75),
        f64::from_bits(0x3fa759af49d494dd),
        f64::from_bits(0x3fa002e1864dda2e),
        f64::from_bits(0x3f97c2d5d468cdd9),
        f64::from_bits(0x3f9292834c025357),
    ],
    [
        f64::from_bits(0x3ff00abe0c129e1e),
        f64::from_bits(0x3c67ceb0ee49d42a),
        f64::from_bits(0x3fc5a3385d5c7ba5),
        f64::from_bits(0x3fb3bf51056f6637),
        f64::from_bits(0x3fa7dba76b124b37),
        f64::from_bits(0x3fa07be4b02e94c4),
        f64::from_bits(0x3f98a6bb92513f01),
        f64::from_bits(0x3f936afd4c615aec),
    ],
    [
        f64::from_bits(0x3ff0102bcffd6acd),
        f64::from_bits(0xbc8c2294c65d2e86),
        f64::from_bits(0x3fc5caff17351901),
        f64::from_bits(0x3fb407abbc04feb3),
        f64::from_bits(0x3fa86179b8005949),
        f64::from_bits(0x3fa0f97520bd4e72),
        f64::from_bits(0x3f99950c5c89f3df),
        f64::from_bits(0x3f944f2344e7b664),
    ],
    [
        f64::from_bits(0x3ff015a397cf0f1c),
        f64::from_bits(0xbc8eebd6ccfe3ee3),
        f64::from_bits(0x3fc5f3581be7b08b),
        f64::from_bits(0x3fb4519ddf1ae531),
        f64::from_bits(0x3fa8eb4b6ee35e92),
        f64::from_bits(0x3fa17bc85414cd46),
        f64::from_bits(0x3f9a8e5895e3fcf9),
        f64::from_bits(0x3f953fafdc629400),
    ],
    [
        f64::from_bits(0x3ff01b2588811eeb),
        f64::from_bits(0x3c47193e5d0a915f),
        f64::from_bits(0x3fc61c46a67205d2),
        f64::from_bits(0x3fb49d33a6eeae0b),
        f64::from_bits(0x3fa979438563c014),
        f64::from_bits(0x3fa20316ae977f05),
        f64::from_bits(0x3f9b9339afb53aa4),
        f64::from_bits(0x3f963d6b02c42d0a),
    ],
    [
        f64::from_bits(0x3ff020b1c7df0575),
        f64::from_bits(0xbc8dd547e329c1e5),
        f64::from_bits(0x3fc645ce0ab901bb),
        f64::from_bits(0x3fb4ea79c34fc7a6),
        f64::from_bits(0x3faa0b8ac08940ec),
        f64::from_bits(0x3fa28f9babd0629b),
        f64::from_bits(0x3f9ca452cf90a55e),
        f64::from_bits(0x3f97492b016730ef),
    ],
    [
        f64::from_bits(0x3ff026487c8c5d71),
        f64::from_bits(0xbc95fd9b68dc3b6e),
        f64::from_bits(0x3fc66ff1b67d5d70),
        f64::from_bits(0x3fb5397d613373eb),
        f64::from_bits(0x3faaa24bce3aeb4a),
        f64::from_bits(0x3fa3219610b150ac),
        f64::from_bits(0x3f9dc251825103f1),
        f64::from_bits(0x3f9863d5a3932532),
    ],
    [
        f64::from_bits(0x3ff02be9ce0b87cd),
        f64::from_bits(0x3c7e5d09da2e0f04),
        f64::from_bits(0x3fc69ab5325bc359),
        f64::from_bits(0x3fb58a4c3097aab3),
        f64::from_bits(0x3fab3db3605f46f2),
        f64::from_bits(0x3fa3b94821742cab),
        f64::from_bits(0x3f9eedee7da72a15),
        f64::from_bits(0x3f998e6179a3e9a0),
    ],
    [
        f64::from_bits(0x3ff03195e4c483f1),
        f64::from_bits(0xbc95db10ad66eacb),
        f64::from_bits(0x3fc6c61c22d908f0),
        f64::from_bits(0x3fb5dcf46ab9f2cb),
        f64::from_bits(0x3fabddf049bb1f4d),
        f64::from_bits(0x3fa456f7db6ac768),
        f64::from_bits(0x3fa013f738bd7bb3),
        f64::from_bits(0x3f9ac9d739783d21),
    ],
    [
        f64::from_bits(0x3ff0374cea0c0c9f),
        f64::from_bits(0xbc9917bff5241c76),
        f64::from_bits(0x3fc6f22a497b2ec0),
        f64::from_bits(0x3fb63184d8a79db5),
        f64::from_bits(0x3fac83339caf946e),
        f64::from_bits(0x3fa4faef331019d4),
        f64::from_bits(0x3fa0b8917547d678),
        f64::from_bits(0x3f9c17533f147e1c),
    ],
    [
        f64::from_bits(0x3ff03d0f082afcc8),
        f64::from_bits(0xbc9018bbcddb49eb),
        f64::from_bits(0x3fc71ee385efdf06),
        f64::from_bits(0x3fb6880cda2d3885),
        f64::from_bits(0x3fad2db0cbfae54d),
        f64::from_bits(0x3fa5a57c56b50c5e),
        f64::from_bits(0x3fa16535a40098b2),
        f64::from_bits(0x3f9d780730b8ebb8),
    ],
    [
        f64::from_bits(0x3ff042dc6a65ffbf),
        f64::from_bits(0xbc8c7ea28dce95d1),
        f64::from_bits(0x3fc74c4bd7412f9e),
        f64::from_bits(0x3fb6e09c6d2b72bc),
        f64::from_bits(0x3faddd9dcda253de),
        f64::from_bits(0x3fa656f1f62b5001),
        f64::from_bits(0x3fa21a5ae2ac77ee),
        f64::from_bits(0x3f9eed3bca067f0e),
    ],
    [
        f64::from_bits(0x3ff048b53d05907b),
        f64::from_bits(0x3c9634fffed6e2a6),
        f64::from_bits(0x3fc77a675d1978be),
        f64::from_bits(0x3fb73b4435583415),
        f64::from_bits(0x3fae9333402ebbf3),
        f64::from_bits(0x3fa70fa78fd9f73f),
        f64::from_bits(0x3fa2d8804d934fe1),
        f64::from_bits(0x3fa03c29691a281c),
    ],
    [
        f64::from_bits(0x3ff04e99ad5e4bcd),
        f64::from_bits(0xbc9e97a72fe827e0),
        f64::from_bits(0x3fc7a93a5917200c),
        f64::from_bits(0x3fb7981584731c05),
        f64::from_bits(0x3faf4eac9268fae2),
        f64::from_bits(0x3fa7cff9c3b19721),
        f64::from_bits(0x3fa3a02d9c1e0145),
        f64::from_bits(0x3fa10d64a0e56953),
    ],
    [
        f64::from_bits(0x3ff05489e9d99995),
        f64::from_bits(0x3c8d177637ec6a2b),
        f64::from_bits(0x3fc7d8c930314681),
        f64::from_bits(0x3fb7f72262f532e4),
        f64::from_bits(0x3fb0082416e39013),
        f64::from_bits(0x3fa8984aac80ddf4),
        f64::from_bits(0x3fa471f3caf18eb8),
        f64::from_bits(0x3fa1eb1cce6dd570),
    ],
    [
        f64::from_bits(0x3ff05a8621feb16b),
        f64::from_bits(0xbc7e5b33b1407c5f),
        f64::from_bits(0x3fc809186c2e57dd),
        f64::from_bits(0x3fb8587d99442dc8),
        f64::from_bits(0x3fb06c23d1dfcb7f),
        f64::from_bits(0x3fa969024036dd22),
        f64::from_bits(0x3fa54e6dd4d2af33),
        f64::from_bits(0x3fa2d62f439f2a31),
    ],
    [
        f64::from_bits(0x3ff0608e867bff30),
        f64::from_bits(0x3c8cbef5d8580027),
        f64::from_bits(0x3fc83a2cbd2d8ba2),
        f64::from_bits(0x3fb8bc3ab9724c6e),
        f64::from_bits(0x3fb0d377ef1e0c39),
        f64::from_bits(0x3faa428eb7addf84),
        f64::from_bits(0x3fa636417bc01ff2),
        f64::from_bits(0x3fa3cf8acc7eb2a0),
    ],
    [
        f64::from_bits(0x3ff066a34930ec8d),
        f64::from_bits(0xbc9480f445fedad1),
        f64::from_bits(0x3fc86c0afb447a74),
        f64::from_bits(0x3fb9226e29948d9c),
        f64::from_bits(0x3fb13e44a9b5a3a6),
        f64::from_bits(0x3fab2564fea8b3fe),
        f64::from_bits(0x3fa72a2023d92458),
        f64::from_bits(0x3fa4d8313cec3485),
    ],
    [
        f64::from_bits(0x3ff06cc49d38146c),
        f64::from_bits(0xbc8b55394f4fc07b),
        f64::from_bits(0x3fc89eb82831feed),
        f64::from_bits(0x3fb98b2d2eb9bb23),
        f64::from_bits(0x3fb1acb01e9ab414),
        f64::from_bits(0x3fac12012cbd00c6),
        f64::from_bits(0x3fa82ac7c1d15c38),
        f64::from_bits(0x3fa5f13925c6edca),
    ],
    [
        f64::from_bits(0x3ff072f2b6f1e601),
        f64::from_bits(0xbc92dcbb05419970),
        f64::from_bits(0x3fc8d2397127aebb),
        f64::from_bits(0x3fb9f68df88da51d),
        f64::from_bits(0x3fb21ee26a4f62a1),
        f64::from_bits(0x3fad08e707f7ae6f),
        f64::from_bits(0x3fa93903dee3feb0),
        f64::from_bits(0x3fa71bcfb5c57b59),
    ],
    [
        f64::from_bits(0x3ff0792dcc0fbd20),
        f64::from_bits(0xbc75bf23ee4f9d54),
        f64::from_bits(0x3fc9069430ab508a),
        f64::from_bits(0x3fba64a7adb4cd85),
        f64::from_bits(0x3fb29505c8b48349),
        f64::from_bits(0x3fae0aa2921cfa60),
        f64::from_bits(0x3faa55aeb46f4322),
        f64::from_bits(0x3fa8593acad3bece),
    ],
    [
        f64::from_bits(0x3ff07f76139f761d),
        f64::from_bits(0x3c9fa1046481bb82),
        f64::from_bits(0x3fc93bcdf091cca6),
        f64::from_bits(0x3fbad59278edc42f),
        f64::from_bits(0x3fb30f46b7261652),
        f64::from_bits(0x3faf17c8a17c843e),
        f64::from_bits(0x3fab81b2619e15b5),
        f64::from_bits(0x3fa9aadb395f5ae4),
    ],
    [
        f64::from_bits(0x3ff085cbc61783c1),
        f64::from_bits(0x3c90a6e9efa20176),
        f64::from_bits(0x3fc971ec6c1531e4),
        f64::from_bits(0x3fbb496797068912),
        f64::from_bits(0x3fb38dd419140184),
        f64::from_bits(0x3fb0187bc3357bbb),
        f64::from_bits(0x3facbe0a3dcafe26),
        f64::from_bits(0x3fab122f4fa499d0),
    ],
    [
        f64::from_bits(0x3ff08c2f1d638e4c),
        f64::from_bits(0x3c7b47c159534a3d),
        f64::from_bits(0x3fc9a8f592078624),
        f64::from_bits(0x3fbbc04165b57ab2),
        f64::from_bits(0x3fb410df5f4bed1d),
        f64::from_bits(0x3fb0ab6bdf478c71),
        f64::from_bits(0x3fae0bc44a945c64),
        f64::from_bits(0x3fac90d59bcd5701),
    ],
    [
        f64::from_bits(0x3ff092a054f1a2fc),
        f64::from_bits(0xbc92f657224e9830),
        f64::from_bits(0x3fc9e0ef87243a2c),
        f64::from_bits(0x3fbc3a3b7366a278),
        f64::from_bits(0x3fb4989cb22e2175),
        f64::from_bits(0x3fb1450e5ba7ad39),
        f64::from_bits(0x3faf6c02c8f0ef93),
        f64::from_bits(0x3fae288ffc8d182c),
    ],
    [
        f64::from_bits(0x3ff0991fa9bffbf4),
        f64::from_bits(0xbc5ca1140a1abbf4),
        f64::from_bits(0x3fca19e0a8823b80),
        f64::from_bits(0x3fbcb772900f9c24),
        f64::from_bits(0x3fb525431f0cbb2e),
        f64::from_bits(0x3fb1e5c2d06804e1),
        f64::from_bits(0x3fb06ffefa7aa6b8),
        f64::from_bits(0x3fafdb4704dca347),
    ],
    [
        f64::from_bits(0x3ff09fad5a6b68f9),
        f64::from_bits(0x3c7aa1f06e92964e),
        f64::from_bits(0x3fca53cf8e28c50e),
        f64::from_bits(0x3fbd3804df1de350),
        f64::from_bits(0x3fb5b70cc8fa98dc),
        f64::from_bits(0x3fb28def298c979b),
        f64::from_bits(0x3fb13482f6347eeb),
        f64::from_bits(0x3fb0d586de48358c),
    ],
    [
        f64::from_bits(0x3ff0a649a73e61f2),
        f64::from_bits(0x3c874ac0d817e9c7),
        f64::from_bits(0x3fca8ec30dc93891),
        f64::from_bits(0x3fbdbc11ea950625),
        f64::from_bits(0x3fb64e371d5616d3),
        f64::from_bits(0x3fb33e0023936249),
        f64::from_bits(0x3fb204426263066a),
        f64::from_bits(0x3fb1cd12e4629723),
    ],
    [
        f64::from_bits(0x3ff0acf4d240ccc4),
        f64::from_bits(0x3c9da890f3b40bd3),
        f64::from_bits(0x3fcacac23da07797),
        f64::from_bits(0x3fbe43bab7741a98),
        f64::from_bits(0x3fb6eb030c631819),
        f64::from_bits(0x3fb3f669d2eb516e),
        f64::from_bits(0x3fb2e0006ae505ae),
        f64::from_bits(0x3fb2d58204457c82),
    ],
    [
        f64::from_bits(0x3ff0b3af1f4880bb),
        f64::from_bits(0x3c7f450fb78d32ba),
        f64::from_bits(0x3fcb07d4778263af),
        f64::from_bits(0x3fbecf21db7be0ef),
        f64::from_bits(0x3fb78db5465013e4),
        f64::from_bits(0x3fb4b7a8376f0996),
        f64::from_bits(0x3fb3c88f9f2ef221),
        f64::from_bits(0x3fb3f02ad9eb9753),
    ],
    [
        f64::from_bits(0x3ff0ba78d40a9260),
        f64::from_bits(0xbc957b07a441e242),
        f64::from_bits(0x3fcb46015c126262),
        f64::from_bits(0x3fbf5e6b94713f3d),
        f64::from_bits(0x3fb836967d0afecf),
        f64::from_bits(0x3fb5823fdd1707b9),
        f64::from_bits(0x3fb4bed355269dc2),
        f64::from_bits(0x3fb51e83065121cf),
    ],
    [
        f64::from_bits(0x3ff0c152382d7366),
        f64::from_bits(0xbc9ee6913347c2a6),
        f64::from_bits(0x3fcb8550d62bfb6e),
        f64::from_bits(0x3fbff1bde0fa3cad),
        f64::from_bits(0x3fb8e5f3ab550989),
        f64::from_bits(0x3fb656be8b38ebaf),
        f64::from_bits(0x3fb5c3c13008a099),
        f64::from_bits(0x3fb662225a1b4f77),
    ],
];

fn fast_two_sum(x: f64, y: f64) -> (f64, f64) {
    let s = x + y;
    (s, y - (s - x))
}

/// Returns a result only when the proven error interval rounds to one value.
pub(crate) fn fast_acos(x: f64) -> Option<f64> {
    let ax = x.abs();
    if ax >= 1.0 || x.is_nan() {
        return None;
    }
    let (t, z, zl, jd, f0h, f0l, eps);
    if ax > 0.5 {
        f0h = if x.is_sign_negative() {
            std::f64::consts::PI
        } else {
            0.0
        };
        f0l = if x.is_sign_negative() {
            f64::from_bits(0x3ca1a62633145c07)
        } else {
            0.0
        };
        let u = 2.0 - 2.0 * ax;
        jd = (u * 32.0).round_ties_even();
        z = u.sqrt().copysign(x);
        zl = z.mul_add(z, -u) * ((-0.5 / u) * z);
        t = 0.25 * u - jd * 0.0078125;
        eps =
            (z * t).abs() * f64::from_bits(0x3cb8c00000000000) + f64::from_bits(0x3960000000000000);
    } else {
        f0h = std::f64::consts::FRAC_PI_2;
        f0l = f64::from_bits(0x3c91a62633145c07);
        if ax <= f64::from_bits(0x3f00000000000000) {
            let v = if ax <= f64::from_bits(0x3c8cb3b3869747f4) {
                0.0
            } else {
                (x * x) * (f64::from_bits(0xbfc5555555555555) * x)
            };
            let (h, w) = fast_two_sum(f0h, -x);
            let l = v + (w + f0l);
            let eps = f64::from_bits(0x3b03400000000000);
            let lb = h + (l - eps);
            let ub = h + (l + eps);
            return if lb == ub { Some(lb) } else { None };
        }
        let square = x * x;
        jd = (square * 128.0).round_ties_even();
        t = x.mul_add(x, -0.0078125 * jd);
        z = -x;
        zl = 0.0;
        eps = (z * t) * f64::from_bits(0x3cb8100000000000);
    }
    let c = COEFF[jd as usize];
    let t2 = t * t;
    let d = t * ((c[2] + t * c[3]) + t2 * ((c[4] + t * c[5]) + t2 * (c[6] + t * c[7])));
    let mut fh = z * c[0];
    let mut fl = (z * (c[1] + d) + zl * c[0]) + z.mul_add(c[0], -fh);
    let (sh, sl) = fast_two_sum(f0h, fh);
    fl = (f0l + fl) + sl;
    fh = sh;
    let lb = fh + (fl - eps);
    let ub = fh + (fl + eps);
    if lb == ub { Some(lb) } else { None }
}

/// `asin(ax)` for `2^-27 <= ax < 1.0` before rounding, as `[hi, lo, eps]`.
///
/// Reuses `COEFF`, which approximates `asin(sqrt(s)) / sqrt(s)` on `s in [0, 0.25]`:
/// - For `ax <= 0.5`: `asin(ax) = ax * P(ax^2)`.
/// - For `0.5 < ax < 1.0`: `asin(ax) = pi/2 - sqrt(2 - 2 ax) * P((1 - ax) / 2)`.
pub(crate) fn fast_asin_parts(ax: f64) -> Option<[f64; 3]> {
    if !(f64::from_bits(0x3e40000000000000)..1.0).contains(&ax) {
        return None;
    }
    if ax > 0.5 {
        let f0h = std::f64::consts::FRAC_PI_2;
        let f0l = f64::from_bits(0x3c91a62633145c07);
        let u = 2.0 - 2.0 * ax;
        let jd = (u * 32.0).round_ties_even();
        let z = -u.sqrt();
        let zl = z.mul_add(z, -u) * ((-0.5 / u) * z);
        let t = 0.25 * u - jd * 0.0078125;
        let c = COEFF[jd as usize];
        let t2 = t * t;
        let d = t * ((c[2] + t * c[3]) + t2 * ((c[4] + t * c[5]) + t2 * (c[6] + t * c[7])));
        let fh = z * c[0];
        let fl = (z * (c[1] + d) + zl * c[0]) + z.mul_add(c[0], -fh);
        let (sh, sl) = fast_two_sum(f0h, fh);
        let (fh, fl) = fast_two_sum(sh, (f0l + fl) + sl);
        let eps =
            (z * t).abs() * f64::from_bits(0x3cc0000000000000) + f64::from_bits(0x39c0000000000000);
        Some([fh, fl, eps])
    } else {
        let square = ax * ax;
        let jd = (square * 128.0).round_ties_even();
        let t = ax.mul_add(ax, -0.0078125 * jd);
        let c = COEFF[jd as usize];
        let t2 = t * t;
        let d = t * ((c[2] + t * c[3]) + t2 * ((c[4] + t * c[5]) + t2 * (c[6] + t * c[7])));
        let fh = ax * c[0];
        let fl = ax * (c[1] + d) + ax.mul_add(c[0], -fh);
        let (fh, fl) = fast_two_sum(fh, fl);
        let eps = (ax * t).abs() * f64::from_bits(0x3cc0000000000000)
            + ax * f64::from_bits(0x39c0000000000000);
        Some([fh, fl, eps])
    }
}

/// Returns `asin(x)` only when the proven error interval rounds to one value.
pub(crate) fn fast_asin(x: f64) -> Option<f64> {
    let ax = x.abs();
    if ax < f64::from_bits(0x3e40000000000000) {
        // For |x| < 2^-27, |asin(x) - x| < x^3 / 5.9 < 2^-56.5 |x| < 0.25 ulp(x),
        // so x is already the correctly rounded value (including -0.0 and subnormals).
        return Some(x);
    }
    let [fh, fl, eps] = fast_asin_parts(ax)?;
    let lb = fh + (fl - eps);
    let ub = fh + (fl + eps);
    if lb == ub { Some(lb.copysign(x)) } else { None }
}
