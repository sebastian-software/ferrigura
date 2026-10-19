/**
 * Writes the table that `ferrigura_model::value::number_text` is checked against: one line per
 * double, its bit pattern and `String(x)` in the running JavaScript engine.
 *
 * The doubles are the special values, the limits of the plain notation (`1e-6` and `1e21`), a
 * power of ten and a negative number with three digits for every decimal exponent, 2000 bit
 * patterns drawn from a fixed generator, and 2000 short decimals scaled to between `1e-15` and
 * `1e25`, which are the numbers that diagrams write.
 *
 * Usage:
 *   tsx tools/port/number-text-table.ts \> crates/ferrigura-model/tests/data/number_text.txt
 */

const view = new DataView(new ArrayBuffer(8));

function bitsOf(value: number): string {
  view.setFloat64(0, value);
  return `0x${view.getBigUint64(0).toString(16).padStart(16, '0')}`;
}

function fromBits(bits: bigint): number {
  view.setBigUint64(0, bits);
  return view.getFloat64(0);
}

const MASK = 0xffff_ffff_ffff_ffffn;
let state = 0x9e37_79b9_7f4a_7c15n;

/** A shift-register generator with a fixed seed, so that the table is the same on every run. */
function next(): bigint {
  state ^= (state << 13n) & MASK;
  state ^= state >> 7n;
  state ^= (state << 17n) & MASK;
  return state;
}

const values: number[] = [
  0,
  -0,
  Number.NaN,
  Infinity,
  -Infinity,
  Number.MIN_VALUE,
  -Number.MIN_VALUE,
  2.2250738585072014e-308,
  Number.MAX_VALUE,
  -Number.MAX_VALUE,
  1e21,
  -1e21,
  999999999999999900000,
  1e-6,
  -1e-6,
  1e-7,
  0.1 + 0.2,
  1 / 3,
  2 ** 53,
  2 ** 53 + 2,
  2 ** 70,
  123456789012345680000,
  1.2345678901234568e21,
  1.8369701987210297e-14,
  -1.8369701987210297e-14,
  6.123233995736766e-17,
  -2.4492935982947064e-16,
  1e-10,
  9.99e-11,
  1.5,
  100,
  1e20,
  1.5e300,
  0.000001234,
  0.0000001234,
  12.5,
  -0.5,
  0.05,
];
for (let exponent = -324; exponent <= 308; exponent++) {
  values.push(Number(`1e${exponent}`), Number(`-1.25e${exponent}`));
}
for (let index = 0; index < 2000; index++) {
  const value = fromBits(next());
  if (Number.isFinite(value)) {
    values.push(value);
  }
}
for (let index = 0; index < 2000; index++) {
  const digits = Number(next() % 1_000_000n) / 1000;
  const power = Number(next() % 40n) - 15;
  const sign = next() % 2n === 0n ? 1 : -1;
  values.push(sign * digits * 10 ** power);
}

const lines = [`# String(x) in node ${process.version} (see tools/port/number-text-table.ts)`];
for (const value of values) {
  lines.push(`${bitsOf(value)} ${String(value)}`);
}
process.stdout.write(`${lines.join('\n')}\n`);
