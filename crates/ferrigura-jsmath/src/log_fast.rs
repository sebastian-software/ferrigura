//! A fast path for `log` and `log10`: table-driven double-double evaluation with a proven error
//! bound, and a rounding test that returns a result only when the whole error interval rounds to one
//! double. When the test is not conclusive, the caller takes the fixed-point evaluator in
//! `correct.rs`, so the result is the correctly rounded value either way. Written for this crate.
//!
//! Every operation uses ordinary rounded double operations (`u = 2^-53`, round to nearest) and
//! error-free transformations (`two_sum` and `two_prod` from `trig_fast.rs`), without fused
//! multiply-adds, so the result does not depend on the target.

use crate::trig_fast::{horner_step, mul_dd, round_test, two_prod, two_sum};

/// `ln 2 = LN2_HI + LN2_LO + delta`, `|delta| < 2^-106`.
pub(crate) const LN2: (f64, f64) = (
    f64::from_bits(0x3fe62e42fefa39ef),
    f64::from_bits(0x3c7abc9e3b39803f),
);

/// `1 / ln 10 = INV_LN10_HI + INV_LN10_LO + delta`, `|delta| < 2^-108`.
pub(crate) const INV_LN10: (f64, f64) = (
    f64::from_bits(0x3fdbcb7b1526e50e),
    f64::from_bits(0x3c695355baaafad3),
);

/// `(-1)^(n-1) / n` for `n = 2..=7` as double-doubles (`hi` nearest double, `lo` nearest to the rest).
pub(crate) const LN_DD: [(f64, f64); 6] = [
    (f64::from_bits(0xbfe0000000000000), 0.0),
    (
        f64::from_bits(0x3fd5555555555555),
        f64::from_bits(0x3c75555555555555),
    ),
    (f64::from_bits(0xbfd0000000000000), 0.0),
    (
        f64::from_bits(0x3fc999999999999a),
        f64::from_bits(0xbc6999999999999a),
    ),
    (
        f64::from_bits(0xbfc5555555555555),
        f64::from_bits(0xbc65555555555555),
    ),
    (
        f64::from_bits(0x3fc2492492492492),
        f64::from_bits(0x3c62492492492492),
    ),
];

/// `(-1)^(n-1) / n` for `n = 8..=13` as doubles.
pub(crate) const LN_TAIL: [f64; 6] = [
    f64::from_bits(0xbfc0000000000000),
    f64::from_bits(0x3fbc71c71c71c71c),
    f64::from_bits(0xbfb999999999999a),
    f64::from_bits(0x3fb745d1745d1746),
    f64::from_bits(0xbfb5555555555555),
    f64::from_bits(0x3fb3b13b13b13b14),
];

/// For `i in 0..128`, `(r_i, hi, lo)` where `r_i = RN(1 / (1 + i / 128))` and
/// `-ln(r_i) = hi + lo + delta` with `|delta| <= u^2 |hi|`.
/// Verified against fixed-point `ln_fx` in the tests of `correct.rs`.
pub(crate) const TABLE: [(f64, f64, f64); 128] = [
    (
        f64::from_bits(0x3ff0000000000000),
        f64::from_bits(0x0000000000000000),
        f64::from_bits(0x0000000000000000),
    ),
    (
        f64::from_bits(0x3fefc07f01fc07f0),
        f64::from_bits(0x3f7fe02a6b106799),
        f64::from_bits(0xbbce44b7e3711e7f),
    ),
    (
        f64::from_bits(0x3fef81f81f81f820),
        f64::from_bits(0x3f8fc0a8b0fc03c4),
        f64::from_bits(0xbc183092c5964281),
    ),
    (
        f64::from_bits(0x3fef44659e4a4271),
        f64::from_bits(0x3f97b91b07d5b126),
        f64::from_bits(0xbc16d80ab38e9430),
    ),
    (
        f64::from_bits(0x3fef07c1f07c1f08),
        f64::from_bits(0x3f9f829b0e7832f8),
        f64::from_bits(0x3c333e3f04f1ef25),
    ),
    (
        f64::from_bits(0x3feecc07b301ecc0),
        f64::from_bits(0x3fa39e87b9febd68),
        f64::from_bits(0xbc45bfa937f551b7),
    ),
    (
        f64::from_bits(0x3fee9131abf0b767),
        f64::from_bits(0x3fa77458f632dcff),
        f64::from_bits(0x3c08d3ca87b92968),
    ),
    (
        f64::from_bits(0x3fee573ac901e574),
        f64::from_bits(0x3fab42dd711971b9),
        f64::from_bits(0x3c40a34531f67db5),
    ),
    (
        f64::from_bits(0x3fee1e1e1e1e1e1e),
        f64::from_bits(0x3faf0a30c01162a8),
        f64::from_bits(0x3c485f325c5bbacd),
    ),
    (
        f64::from_bits(0x3fede5d6e3f8868a),
        f64::from_bits(0x3fb16536eea37ae3),
        f64::from_bits(0x3c52189705cf74ca),
    ),
    (
        f64::from_bits(0x3fedae6076b981db),
        f64::from_bits(0x3fb341d7961bd1d0),
        f64::from_bits(0xbc53599f227becbb),
    ),
    (
        f64::from_bits(0x3fed77b654b82c34),
        f64::from_bits(0x3fb51b073f06183c),
        f64::from_bits(0xbc55b61c65e5741a),
    ),
    (
        f64::from_bits(0x3fed41d41d41d41d),
        f64::from_bits(0x3fb6f0d28ae56b4e),
        f64::from_bits(0xbc420db323097324),
    ),
    (
        f64::from_bits(0x3fed0cb58f6ec074),
        f64::from_bits(0x3fb8c345d6319b23),
        f64::from_bits(0xbc5294d2f5668495),
    ),
    (
        f64::from_bits(0x3fecd85689039b0b),
        f64::from_bits(0x3fba926d3a4ad562),
        f64::from_bits(0xbc4d7a16eab1e2ad),
    ),
    (
        f64::from_bits(0x3feca4b3055ee191),
        f64::from_bits(0x3fbc5e548f5bc743),
        f64::from_bits(0x3c42eb0bf7c0b0d9),
    ),
    (
        f64::from_bits(0x3fec71c71c71c71c),
        f64::from_bits(0x3fbe27076e2af2ea),
        f64::from_bits(0xbc361578001e015a),
    ),
    (
        f64::from_bits(0x3fec3f8f01c3f8f0),
        f64::from_bits(0x3fbfec9131dbeabc),
        f64::from_bits(0xbc55746b9981b36c),
    ),
    (
        f64::from_bits(0x3fec0e070381c0e0),
        f64::from_bits(0x3fc0d77e7cd08e5b),
        f64::from_bits(0x3c69a5dc5e9030ad),
    ),
    (
        f64::from_bits(0x3febdd2b899406f7),
        f64::from_bits(0x3fc1b72ad52f67a2),
        f64::from_bits(0xbc6fbe7ee5c69946),
    ),
    (
        f64::from_bits(0x3febacf914c1bad0),
        f64::from_bits(0x3fc29552f81ff521),
        f64::from_bits(0x3c6301771c407dc0),
    ),
    (
        f64::from_bits(0x3feb7d6c3dda338b),
        f64::from_bits(0x3fc371fc201e8f75),
        f64::from_bits(0x3c1e6cb62af18a02),
    ),
    (
        f64::from_bits(0x3feb4e81b4e81b4f),
        f64::from_bits(0x3fc44d2b6ccb7d1c),
        f64::from_bits(0x3c47d3d950f87e23),
    ),
    (
        f64::from_bits(0x3feb2036406c80d9),
        f64::from_bits(0x3fc526e5e3a1b438),
        f64::from_bits(0xbc6546ff8a470d3a),
    ),
    (
        f64::from_bits(0x3feaf286bca1af28),
        f64::from_bits(0x3fc5ff3070a793d6),
        f64::from_bits(0xbc5bc60efafc6f6c),
    ),
    (
        f64::from_bits(0x3feac5701ac5701b),
        f64::from_bits(0x3fc6d60fe719d21b),
        f64::from_bits(0x3c6d551d97132e87),
    ),
    (
        f64::from_bits(0x3fea98ef606a63be),
        f64::from_bits(0x3fc7ab890210d907),
        f64::from_bits(0xbc61072534a57e7d),
    ),
    (
        f64::from_bits(0x3fea6d01a6d01a6d),
        f64::from_bits(0x3fc87fa06520c911),
        f64::from_bits(0xbc69f7fdbfa08d9a),
    ),
    (
        f64::from_bits(0x3fea41a41a41a41a),
        f64::from_bits(0x3fc9525a9cf456b6),
        f64::from_bits(0xbc626fb3e2b1d1da),
    ),
    (
        f64::from_bits(0x3fea16d3f97a4b02),
        f64::from_bits(0x3fca23bc1fe2b561),
        f64::from_bits(0x3c624dc46c1ea664),
    ),
    (
        f64::from_bits(0x3fe9ec8e951033d9),
        f64::from_bits(0x3fcaf3c94e80bff3),
        f64::from_bits(0x3c6a3398064df33e),
    ),
    (
        f64::from_bits(0x3fe9c2d14ee4a102),
        f64::from_bits(0x3fcbc286742d8cd4),
        f64::from_bits(0x3c5cfce744870f57),
    ),
    (
        f64::from_bits(0x3fe999999999999a),
        f64::from_bits(0x3fcc8ff7c79a9a20),
        f64::from_bits(0xbc64f689f8434011),
    ),
    (
        f64::from_bits(0x3fe970e4f80cb872),
        f64::from_bits(0x3fcd5c216b4fbb94),
        f64::from_bits(0xbc5a37794d03657d),
    ),
    (
        f64::from_bits(0x3fe948b0fcd6e9e0),
        f64::from_bits(0x3fce27076e2af2e8),
        f64::from_bits(0xbc461578001e015e),
    ),
    (
        f64::from_bits(0x3fe920fb49d0e229),
        f64::from_bits(0x3fcef0adcbdc5935),
        f64::from_bits(0x3c6e8637950dc20d),
    ),
    (
        f64::from_bits(0x3fe8f9c18f9c18fa),
        f64::from_bits(0x3fcfb9186d5e3e29),
        f64::from_bits(0x3c6355519b0de535),
    ),
    (
        f64::from_bits(0x3fe8d3018d3018d3),
        f64::from_bits(0x3fd0402594b4d041),
        f64::from_bits(0xbc608ec217a5022d),
    ),
    (
        f64::from_bits(0x3fe8acb90f6bf3aa),
        f64::from_bits(0x3fd0a324e27390e2),
        f64::from_bits(0x3c7bdcfde8061c03),
    ),
    (
        f64::from_bits(0x3fe886e5f0abb04a),
        f64::from_bits(0x3fd1058bf9ae4ad4),
        f64::from_bits(0x3c03f415699663ec),
    ),
    (
        f64::from_bits(0x3fe8618618618618),
        f64::from_bits(0x3fd1675cababa60f),
        f64::from_bits(0x3c2ce63eab883727),
    ),
    (
        f64::from_bits(0x3fe83c977ab2bedd),
        f64::from_bits(0x3fd1c898c16999fb),
        f64::from_bits(0x3c79f1a39d500e3c),
    ),
    (
        f64::from_bits(0x3fe8181818181818),
        f64::from_bits(0x3fd22941fbcf7966),
        f64::from_bits(0xbc5dbd7ac258a2bd),
    ),
    (
        f64::from_bits(0x3fe7f405fd017f40),
        f64::from_bits(0x3fd2895a13de86a4),
        f64::from_bits(0x3c77ad24c13f040f),
    ),
    (
        f64::from_bits(0x3fe7d05f417d05f4),
        f64::from_bits(0x3fd2e8e2bae11d31),
        f64::from_bits(0xbc61e99b72bd7bf2),
    ),
    (
        f64::from_bits(0x3fe7ad2208e0ecc3),
        f64::from_bits(0x3fd347dd9a987d56),
        f64::from_bits(0xbc716ea62c048cfb),
    ),
    (
        f64::from_bits(0x3fe78a4c8178a4c8),
        f64::from_bits(0x3fd3a64c556945ea),
        f64::from_bits(0x3c3cbcd735d03424),
    ),
    (
        f64::from_bits(0x3fe767dce434a9b1),
        f64::from_bits(0x3fd404308686a7e4),
        f64::from_bits(0xbc6f79f6c1059cdb),
    ),
    (
        f64::from_bits(0x3fe745d1745d1746),
        f64::from_bits(0x3fd4618bc21c5ec2),
        f64::from_bits(0xbc27a42642661c62),
    ),
    (
        f64::from_bits(0x3fe724287f46debc),
        f64::from_bits(0x3fd4be5f957778a1),
        f64::from_bits(0xbc54b366b609027a),
    ),
    (
        f64::from_bits(0x3fe702e05c0b8170),
        f64::from_bits(0x3fd51aad872df82e),
        f64::from_bits(0xbc7d8db0a7cc1543),
    ),
    (
        f64::from_bits(0x3fe6e1f76b4337c7),
        f64::from_bits(0x3fd5767717455a6c),
        f64::from_bits(0xbc6fb2a49af933e8),
    ),
    (
        f64::from_bits(0x3fe6c16c16c16c17),
        f64::from_bits(0x3fd5d1bdbf5809ca),
        f64::from_bits(0xbc77dc9c7c23801f),
    ),
    (
        f64::from_bits(0x3fe6a13cd1537290),
        f64::from_bits(0x3fd62c82f2b9c796),
        f64::from_bits(0xbc5090a0dd59fe35),
    ),
    (
        f64::from_bits(0x3fe6816816816817),
        f64::from_bits(0x3fd686c81e9b14ad),
        f64::from_bits(0x3c7710af840538e3),
    ),
    (
        f64::from_bits(0x3fe661ec6a5122f9),
        f64::from_bits(0x3fd6e08eaa2ba1e4),
        f64::from_bits(0xbc7bfb1b39ca3a0f),
    ),
    (
        f64::from_bits(0x3fe642c8590b2164),
        f64::from_bits(0x3fd739d7f6bbd007),
        f64::from_bits(0x3c5ce24c53fad3f0),
    ),
    (
        f64::from_bits(0x3fe623fa77016240),
        f64::from_bits(0x3fd792a55fdd47a1),
        f64::from_bits(0x3c7f057691fe9ed7),
    ),
    (
        f64::from_bits(0x3fe6058160581606),
        f64::from_bits(0x3fd7eaf83b82afc2),
        f64::from_bits(0xbc4698b43096b576),
    ),
    (
        f64::from_bits(0x3fe5e75bb8d015e7),
        f64::from_bits(0x3fd842d1da1e8b18),
        f64::from_bits(0x3c754ec519784677),
    ),
    (
        f64::from_bits(0x3fe5c9882b931057),
        f64::from_bits(0x3fd89a3386c1425b),
        f64::from_bits(0x3c62d38c40881e0b),
    ),
    (
        f64::from_bits(0x3fe5ac056b015ac0),
        f64::from_bits(0x3fd8f11e873662c8),
        f64::from_bits(0x3c7f85da755a61a3),
    ),
    (
        f64::from_bits(0x3fe58ed2308158ed),
        f64::from_bits(0x3fd947941c2116fb),
        f64::from_bits(0x3c61266e8a3e8838),
    ),
    (
        f64::from_bits(0x3fe571ed3c506b3a),
        f64::from_bits(0x3fd99d958117e08a),
        f64::from_bits(0xbc7315b444ee1f38),
    ),
    (
        f64::from_bits(0x3fe5555555555555),
        f64::from_bits(0x3fd9f323ecbf984d),
        f64::from_bits(0xbc4a92e513217f58),
    ),
    (
        f64::from_bits(0x3fe5390948f40feb),
        f64::from_bits(0x3fda484090e5bb09),
        f64::from_bits(0x3c7fff29adc3ad3b),
    ),
    (
        f64::from_bits(0x3fe51d07eae2f815),
        f64::from_bits(0x3fda9cec9a9a084a),
        f64::from_bits(0xbc5ab7b00ad0dabc),
    ),
    (
        f64::from_bits(0x3fe5015015015015),
        f64::from_bits(0x3fdaf1293247786b),
        f64::from_bits(0x3c5533844a15dc28),
    ),
    (
        f64::from_bits(0x3fe4e5e0a72f0539),
        f64::from_bits(0x3fdb44f77bcc8f64),
        f64::from_bits(0xbc2a0892a8b38eed),
    ),
    (
        f64::from_bits(0x3fe4cab88725af6e),
        f64::from_bits(0x3fdb9858969310fd),
        f64::from_bits(0xbc6f3827583b8877),
    ),
    (
        f64::from_bits(0x3fe4afd6a052bf5b),
        f64::from_bits(0x3fdbeb4d9da71b7a),
        f64::from_bits(0x3c7be1874deaef08),
    ),
    (
        f64::from_bits(0x3fe49539e3b2d067),
        f64::from_bits(0x3fdc3dd7a7cdad4d),
        f64::from_bits(0x3c67d9e0a5bd4d37),
    ),
    (
        f64::from_bits(0x3fe47ae147ae147b),
        f64::from_bits(0x3fdc8ff7c79a9a21),
        f64::from_bits(0x3c73097607bcbfee),
    ),
    (
        f64::from_bits(0x3fe460cbc7f5cf9a),
        f64::from_bits(0x3fdce1af0b85f3ec),
        f64::from_bits(0xbc66416a1aa97b31),
    ),
    (
        f64::from_bits(0x3fe446f86562d9fb),
        f64::from_bits(0x3fdd32fe7e00ebd5),
        f64::from_bits(0x3c64ef6465f5f46e),
    ),
    (
        f64::from_bits(0x3fe42d6625d51f87),
        f64::from_bits(0x3fdd83e7258a2f3e),
        f64::from_bits(0x3c5c515ba2ec9444),
    ),
    (
        f64::from_bits(0x3fe4141414141414),
        f64::from_bits(0x3fddd46a04c1c4a1),
        f64::from_bits(0xbc119d95b62e2476),
    ),
    (
        f64::from_bits(0x3fe3fb013fb013fb),
        f64::from_bits(0x3fde24881a7c6c26),
        f64::from_bits(0x3c605ec7a2caa523),
    ),
    (
        f64::from_bits(0x3fe3e22cbce4a902),
        f64::from_bits(0x3fde744261d68789),
        f64::from_bits(0x3c7cdf68dbcf2ed3),
    ),
    (
        f64::from_bits(0x3fe3c995a47babe7),
        f64::from_bits(0x3fdec399d2468cc1),
        f64::from_bits(0xbc494623581958cf),
    ),
    (
        f64::from_bits(0x3fe3b13b13b13b14),
        f64::from_bits(0x3fdf128f5faf06ec),
        f64::from_bits(0xbc7328df13bb38c2),
    ),
    (
        f64::from_bits(0x3fe3991c2c187f63),
        f64::from_bits(0x3fdf6123fa7028ad),
        f64::from_bits(0x3c55456c3cb6cd06),
    ),
    (
        f64::from_bits(0x3fe3813813813814),
        f64::from_bits(0x3fdfaf588f78f31d),
        f64::from_bits(0x3c6cd7d9f2754362),
    ),
    (
        f64::from_bits(0x3fe3698df3de0748),
        f64::from_bits(0x3fdffd2e0857f497),
        f64::from_bits(0xbc44d05f9366f27f),
    ),
    (
        f64::from_bits(0x3fe3521cfb2b78c1),
        f64::from_bits(0x3fe02552a5a5d0ff),
        f64::from_bits(0x3c6e9c695d7ee800),
    ),
    (
        f64::from_bits(0x3fe33ae45b57bcb2),
        f64::from_bits(0x3fe04bdf9da926d2),
        f64::from_bits(0x3c78fe60804593bf),
    ),
    (
        f64::from_bits(0x3fe323e34a2b10bf),
        f64::from_bits(0x3fe0723e5c1cdf41),
        f64::from_bits(0xbc46a1a71dbba44e),
    ),
    (
        f64::from_bits(0x3fe30d190130d190),
        f64::from_bits(0x3fe0986f4f573521),
        f64::from_bits(0xbc737012b5805e02),
    ),
    (
        f64::from_bits(0x3fe2f684bda12f68),
        f64::from_bits(0x3fe0be72e4252a83),
        f64::from_bits(0x3c7b4c4bdd99efff),
    ),
    (
        f64::from_bits(0x3fe2e025c04b8097),
        f64::from_bits(0x3fe0e44985d1cc8c),
        f64::from_bits(0xbc4c546885a5a707),
    ),
    (
        f64::from_bits(0x3fe2c9fb4d812ca0),
        f64::from_bits(0x3fe109f39e2d4c96),
        f64::from_bits(0x3c8f78fb26c2de46),
    ),
    (
        f64::from_bits(0x3fe2b404ad012b40),
        f64::from_bits(0x3fe12f719593efbd),
        f64::from_bits(0xbc767f6e731c1795),
    ),
    (
        f64::from_bits(0x3fe29e4129e4129e),
        f64::from_bits(0x3fe154c3d2f4d5ea),
        f64::from_bits(0x3c698f33a3965e29),
    ),
    (
        f64::from_bits(0x3fe288b01288b013),
        f64::from_bits(0x3fe179eabbd899a0),
        f64::from_bits(0xbc5c73e320bf059f),
    ),
    (
        f64::from_bits(0x3fe27350b8812735),
        f64::from_bits(0x3fe19ee6b467c96f),
        f64::from_bits(0xbc6fa3422887e218),
    ),
    (
        f64::from_bits(0x3fe25e22708092f1),
        f64::from_bits(0x3fe1c3b81f713c25),
        f64::from_bits(0xbc70b583899021d1),
    ),
    (
        f64::from_bits(0x3fe2492492492492),
        f64::from_bits(0x3fe1e85f5e7040d1),
        f64::from_bits(0xbc8084e99683070e),
    ),
    (
        f64::from_bits(0x3fe23456789abcdf),
        f64::from_bits(0x3fe20cdcd192ab6e),
        f64::from_bits(0xbc8aabf0bc229014),
    ),
    (
        f64::from_bits(0x3fe21fb78121fb78),
        f64::from_bits(0x3fe23130d7bebf43),
        f64::from_bits(0xbc8748725e374d6e),
    ),
    (
        f64::from_bits(0x3fe20b470c67c0d9),
        f64::from_bits(0x3fe2555bce98f7ca),
        f64::from_bits(0x3c89810eb6b440f4),
    ),
    (
        f64::from_bits(0x3fe1f7047dc11f70),
        f64::from_bits(0x3fe2795e1289b11b),
        f64::from_bits(0x3c8ade0fcf6e5a1d),
    ),
    (
        f64::from_bits(0x3fe1e2ef3b3fb874),
        f64::from_bits(0x3fe29d37fec2b08b),
        f64::from_bits(0x3c801735b2e9733f),
    ),
    (
        f64::from_bits(0x3fe1cf06ada2811d),
        f64::from_bits(0x3fe2c0e9ed448e8c),
        f64::from_bits(0xbc88a158f3917586),
    ),
    (
        f64::from_bits(0x3fe1bb4a4046ed29),
        f64::from_bits(0x3fe2e47436e40268),
        f64::from_bits(0x3c80950861a4886b),
    ),
    (
        f64::from_bits(0x3fe1a7b9611a7b96),
        f64::from_bits(0x3fe307d7334f10be),
        f64::from_bits(0x3c7fdac850fab36d),
    ),
    (
        f64::from_bits(0x3fe19453808ca29c),
        f64::from_bits(0x3fe32b1339121d71),
        f64::from_bits(0x3c7d02ab5b3d916b),
    ),
    (
        f64::from_bits(0x3fe1811811811812),
        f64::from_bits(0x3fe34e289d9ce1d2),
        f64::from_bits(0x3c7775c96c42e729),
    ),
    (
        f64::from_bits(0x3fe16e0689427379),
        f64::from_bits(0x3fe37117b54747b6),
        f64::from_bits(0xbc8808bf6deec882),
    ),
    (
        f64::from_bits(0x3fe15b1e5f75270d),
        f64::from_bits(0x3fe393e0d3562a1a),
        f64::from_bits(0xbc838eef67f2483a),
    ),
    (
        f64::from_bits(0x3fe1485f0e0acd3b),
        f64::from_bits(0x3fe3b68449fffc23),
        f64::from_bits(0x3c8c63b7b06164da),
    ),
    (
        f64::from_bits(0x3fe135c81135c811),
        f64::from_bits(0x3fe3d9026a7156fb),
        f64::from_bits(0x3c50084c7a15a4f5),
    ),
    (
        f64::from_bits(0x3fe12358e75d3033),
        f64::from_bits(0x3fe3fb5b84d16f43),
        f64::from_bits(0x3c70a74ea82e55df),
    ),
    (
        f64::from_bits(0x3fe1111111111111),
        f64::from_bits(0x3fe41d8fe84672af),
        f64::from_bits(0xbc8ee6d0cf42e7fa),
    ),
    (
        f64::from_bits(0x3fe0fef010fef011),
        f64::from_bits(0x3fe43f9fe2f9ce67),
        f64::from_bits(0x3c8e1c9ee6d83b86),
    ),
    (
        f64::from_bits(0x3fe0ecf56be69c90),
        f64::from_bits(0x3fe4618bc21c5ec2),
        f64::from_bits(0x3c7e85bd9bd99e3a),
    ),
    (
        f64::from_bits(0x3fe0db20a88f4696),
        f64::from_bits(0x3fe48353d1ea88df),
        f64::from_bits(0xbc840a85d133f80b),
    ),
    (
        f64::from_bits(0x3fe0c9714fbcda3b),
        f64::from_bits(0x3fe4a4f85db03ebb),
        f64::from_bits(0xbc8d76102e1644f2),
    ),
    (
        f64::from_bits(0x3fe0b7e6ec259dc8),
        f64::from_bits(0x3fe4c679afccee39),
        f64::from_bits(0xbc6e971322ce7900),
    ),
    (
        f64::from_bits(0x3fe0a6810a6810a7),
        f64::from_bits(0x3fe4e7d811b75bb0),
        f64::from_bits(0xbc85d3d9ea6e9ea8),
    ),
    (
        f64::from_bits(0x3fe0953f39010954),
        f64::from_bits(0x3fe50913cc01686b),
        f64::from_bits(0x3c79e59d2d85ab62),
    ),
    (
        f64::from_bits(0x3fe0842108421084),
        f64::from_bits(0x3fe52a2d265bc5ab),
        f64::from_bits(0x3c773be4578ad97b),
    ),
    (
        f64::from_bits(0x3fe073260a47f7c6),
        f64::from_bits(0x3fe54b2467999498),
        f64::from_bits(0x3c8f4550a2d0f60c),
    ),
    (
        f64::from_bits(0x3fe0624dd2f1a9fc),
        f64::from_bits(0x3fe56bf9d5b3f399),
        f64::from_bits(0x3c611c6217363fcb),
    ),
    (
        f64::from_bits(0x3fe05197f7d73404),
        f64::from_bits(0x3fe58cadb5cd7989),
        f64::from_bits(0x3c8624bc9764c22c),
    ),
    (
        f64::from_bits(0x3fe0410410410410),
        f64::from_bits(0x3fe5ad404c359f2d),
        f64::from_bits(0x3c8eca6aa97c08e7),
    ),
    (
        f64::from_bits(0x3fe03091b51f5e1a),
        f64::from_bits(0x3fe5cdb1dc6c1765),
        f64::from_bits(0x3c747b71e2eb8419),
    ),
    (
        f64::from_bits(0x3fe0204081020408),
        f64::from_bits(0x3fe5ee02a9241676),
        f64::from_bits(0xbc8bca7da80b6f7e),
    ),
    (
        f64::from_bits(0x3fe0101010101010),
        f64::from_bits(0x3fe60e32f44788d9),
        f64::from_bits(0xbc658376a5f4b135),
    ),
];

const TWO_M92: f64 = f64::from_bits(0x3a30000000000000);
const TWO_M96: f64 = f64::from_bits(0x39f0000000000000);
const TWO_M98: f64 = f64::from_bits(0x39d0000000000000);
const TWO_M102: f64 = f64::from_bits(0x3990000000000000);
const TWO_P54: f64 = f64::from_bits(0x4350000000000000);

/// `ln(x)` before rounding as `[hi, lo, eps]` for a finite positive `x != 1.0`.
///
/// Reduces `x = 2^k * m` with `m in [1 - 2^-9, 2 - 2^-8)` and `idx in 0..128` so that
/// `z = m * r_idx - 1` is computed exactly (`two_prod` + Sterbenz subtraction + `two_sum`) with
/// `|z| <= 2^-8 + 2^-52`. Evaluates `ln(1 + z) = z + z^2 P(z)` to within `2^-103 |z|`, and
/// combines with `k * ln 2 - ln r_idx` (which is exact zero when `k == 0 && idx == 0`, avoiding
/// cancellation near `x = 1`).
pub(crate) fn log_parts(x: f64) -> [f64; 3] {
    let (scaled, extra) = if x < f64::MIN_POSITIVE {
        (x * TWO_P54, -54i64)
    } else {
        (x, 0i64)
    };
    let bits = scaled.to_bits();
    let b = bits.wrapping_add(1u64 << 44);
    let k = ((b >> 52) & 0x7FF) as i64 - 1023 + extra;
    let idx = ((b >> 45) & 127) as usize;
    let mantissa_exp = if (b >> 52) != (bits >> 52) {
        1022u64 << 52
    } else {
        1023u64 << 52
    };
    let m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | mantissa_exp);

    let (r, th, tl) = TABLE[idx];
    let (zh, zl) = if idx == 0 {
        (m - 1.0, 0.0)
    } else {
        let (ph, pl) = two_prod(m, r);
        two_sum(ph - 1.0, pl)
    };

    // z^2 = z2h + z2l with |z2l| <= 3.02 u z2h and error <= 6.01 u^2 zh^2.
    let (z2h, t2) = two_prod(zh, zh);
    let z2 = (z2h, t2 + 2.0 * (zh * zl));

    // P(z) = sum_{n=2}^{13} (-1)^(n-1) / n * z^(n-2).
    let mut tail = 0.0;
    for &c in LN_TAIL.iter().rev() {
        tail = c + zh * tail;
    }
    let mut p = (tail, 0.0);
    for &c in LN_DD.iter().rev() {
        p = horner_step(c, (zh, zl), p);
    }

    // ln(1 + z) = z + z^2 * P(z).
    let (wh, wl) = mul_dd(z2, p);
    let (sh, se) = two_sum(zh, wh);
    let sl = se + (zl + wl);

    if k == 0 && idx == 0 {
        let (fh, fl) = two_sum(sh, sl);
        return [fh, fl, TWO_M96 * z2h + TWO_M102 * fh.abs()];
    }

    let kf = k as f64;
    let (kh, kl1) = two_prod(kf, LN2.0);
    let kl = kl1 + kf * LN2.1;
    let (bh, be) = two_sum(kh, th);
    let bl = be + (kl + tl);
    let (fh0, fe) = two_sum(bh, sh);
    let fl0 = fe + (bl + sl);
    let (fh, fl) = two_sum(fh0, fl0);
    let eps = TWO_M92 * (kf.abs() + 1.0);
    [fh, fl, eps]
}

/// `log10(x)` before rounding as `[hi, lo, eps]` for a finite positive `x != 1.0`.
pub(crate) fn log10_parts(x: f64) -> [f64; 3] {
    let [lh, ll, leps] = log_parts(x);
    let (ph, pl) = mul_dd((lh, ll), INV_LN10);
    let (fh, fl) = two_sum(ph, pl);
    let eps = leps + TWO_M98 * fh.abs();
    [fh, fl, eps]
}

/// `log(x)` for a finite positive `x != 1.0`, correctly rounded, or `None` for fixed-point fallback.
pub(crate) fn log(x: f64) -> Option<f64> {
    round_test(log_parts(x))
}

/// `log10(x)` for a finite positive `x != 1.0`, correctly rounded, or `None` for fixed-point fallback.
pub(crate) fn log10(x: f64) -> Option<f64> {
    round_test(log10_parts(x))
}
