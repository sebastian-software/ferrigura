/**
 * Probes `ferrigura-jsmath` against JavaScript's `Math` functions, bit for bit.
 *
 * Three steps, each run on the same inputs:
 *
 *   1. `generate <dir>` writes the inputs of each function (a million per function, in segments
 *      that cover the argument ranges of the layouts, the special values and the thresholds of
 *      fdlibm), and the results of `Math` on them, in the runtime given by `--runtime`: `node`
 *      (the default) or `chromium` (Playwright's Chromium, which the geometry checks render in).
 *   2. `cargo run --release -p ferrigura-jsmath --example evaluate -- <dir>` evaluates the crate
 *      on the same inputs.
 *   3. `compare <dir>` reports the segments in which the two differ, and exits with 1 if any do.
 *
 * `sample <dir> <out>` writes a deterministic sample of the results as test data for the crate
 * (`crates/ferrigura-jsmath/tests/data`). The sample is read by the crate's tests, which need no
 * JavaScript runtime.
 *
 * Files in `<dir>`, for each function `f`: `f.in` (little-endian `u64` bit patterns, one per
 * input, or two per input for `atan2`, `pow` and `hypot`), `f.v8` (the results of `Math`), `f.rs`
 * (the results of the crate), and `segments.json` (the segments of each function and their
 * starting index). A NaN result is compared with any NaN, since JavaScript cannot observe a NaN's
 * payload; every other result must match in all 64 bits, the sign of a zero included.
 *
 * Usage:
 *   tsx tools/port/jsmath-probe.ts generate <dir> [--runtime node|chromium] [--count N]
 *   tsx tools/port/jsmath-probe.ts compare <dir>
 *   tsx tools/port/jsmath-probe.ts sample <dir> <out-dir> [--per-segment K]
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { endianness } from 'node:os';
import { join } from 'node:path';
import { chromium, type Browser, type Page } from 'playwright';

const UNARY = ['sin', 'cos', 'tan', 'asin', 'acos', 'atan', 'log', 'log10'] as const;
const BINARY = ['atan2', 'pow', 'hypot'] as const;
type UnaryName = (typeof UNARY)[number];
type BinaryName = (typeof BINARY)[number];
type FunctionName = UnaryName | BinaryName;

/** The functions in the order they were added to the probe: the seed of each function's inputs is its
 * position here, so that adding a function leaves the inputs of the others as they were. */
const SEED_ORDER: readonly FunctionName[] = [
  'sin',
  'cos',
  'tan',
  'asin',
  'acos',
  'atan',
  'log',
  'atan2',
  'pow',
  'hypot',
  'log10',
];

const DEFAULT_COUNT = 1_000_000;
const TWO_PI = 2 * Math.PI;
const PI_OVER_TWO = Math.PI / 2;
const CHROME_CHUNK = 100_000;

/** The Float64 view of a bit pattern, read and written as big-endian words so no view is needed. */
const scratch = new DataView(new ArrayBuffer(8));

function fromWords(high: number, low: number): number {
  scratch.setUint32(0, high >>> 0, false);
  scratch.setUint32(4, low >>> 0, false);
  return scratch.getFloat64(0, false);
}

/** `x` with the magnitude moved `steps` units in the last place towards larger magnitudes. */
function stepUlps(x: number, steps: number): number {
  if (!Number.isFinite(x) || x === 0) {
    return x;
  }
  scratch.setFloat64(0, x, false);
  const bits = scratch.getBigUint64(0, false);
  const sign = bits >> 63n;
  const magnitude = bits & ((1n << 63n) - 1n);
  const moved = sign === 0n ? magnitude + BigInt(steps) : magnitude - BigInt(steps);
  if (moved < 0n) {
    return 0;
  }
  scratch.setBigUint64(0, (sign << 63n) | moved, false);
  return scratch.getFloat64(0, false);
}

/** A 32-bit generator (mulberry32), so that the inputs are the same in every runtime. */
function makeRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return (t ^ (t >>> 14)) >>> 0;
  };
}

type Random = () => number;

/** A uniform double in `[0, 1)`. */
function unit(random: Random): number {
  return random() / 4294967296;
}

function uniform(random: Random, low: number, high: number): number {
  return low + unit(random) * (high - low);
}

function uniformInt(random: Random, low: number, high: number): number {
  return low + Math.floor(unit(random) * (high - low + 1));
}

/** A double with any bit pattern, including NaNs and infinities. */
function anyBits(random: Random): number {
  return fromWords(random(), random());
}

/** A double whose exponent field lies in `[lowExponent, highExponent]`, with a random sign. */
function withExponent(random: Random, lowExponent: number, highExponent: number): number {
  const exponent = uniformInt(random, lowExponent, highExponent);
  const sign = random() & 1 ? 0x8000_0000 : 0;
  return fromWords(sign | (exponent << 20) | (random() & 0xfffff), random());
}

/** A value of a fdlibm threshold: the high word `threshold + offset`, with a random low word. */
function nearThreshold(random: Random, thresholds: readonly number[]): number {
  const threshold = thresholds[uniformInt(random, 0, thresholds.length - 1)] ?? 0;
  const offset = uniformInt(random, -2, 2);
  const sign = random() & 1 ? 0x8000_0000 : 0;
  return fromWords(sign | ((threshold + offset) >>> 0), random());
}

/** The specials, which every function is probed on. */
const SPECIALS: readonly number[] = [
  0,
  -0,
  Infinity,
  -Infinity,
  NaN,
  1,
  -1,
  2,
  -2,
  0.5,
  -0.5,
  0.25,
  3,
  10,
  100,
  1e-300,
  -1e-300,
  1e300,
  -1e300,
  Number.MIN_VALUE,
  -Number.MIN_VALUE,
  2.2250738585072014e-308,
  Number.MAX_VALUE,
  -Number.MAX_VALUE,
  Math.PI,
  -Math.PI,
  PI_OVER_TWO,
  -PI_OVER_TWO,
  Math.PI / 4,
  -Math.PI / 4,
  TWO_PI,
  0.3,
  0.78125,
  0.975,
  1.5,
  2.5,
  1 / 3,
  2 ** -27,
  -(2 ** -27),
  2 ** -28,
  2 ** 20,
  2 ** 53,
  2 ** 66,
  2 ** -54,
  2 ** 54,
  2 ** 1023,
  2 ** -1022,
  1 - 2 ** -53,
  1 + 2 ** -52,
  0.5 - 2 ** -54,
  1e4,
  -1e4,
  2 ** 60,
  2 ** -60,
  1e6,
  Math.SQRT1_2,
  Math.SQRT2,
  Math.E,
  7.5e-6,
];

/** The high words where fdlibm changes branch, for the trigonometric functions. */
const TRIG_THRESHOLDS = [
  0x3e30_0000, 0x3e40_0000, 0x3fd3_3333, 0x3fe6_0000, 0x3fe9_0000, 0x3fe9_21fb, 0x3fe5_9428,
  0x4002_d97c, 0x4139_21fb, 0x7ff0_0000,
];
const INVERSE_THRESHOLDS = [0x3c60_0000, 0x3e40_0000, 0x3fe0_0000, 0x3fef_3333];
const ATAN_THRESHOLDS = [
  0x3e30_0000, 0x3e40_0000, 0x3fdc_0000, 0x3fe6_0000, 0x3ff3_0000, 0x4003_8000, 0x4410_0000,
];
const LOG_THRESHOLDS = [0x3fe6_a09e, 0x3ff6_a09e, 0x0010_0000, 0x7ff0_0000];
/** The high words of 1/2, 1 and 2, where fdlibm's `log10` changes how it splits off the exponent. */
const LOG10_THRESHOLDS = [0x3fe0_0000, 0x3ff0_0000, 0x4000_0000, 0x0010_0000, 0x7ff0_0000];

interface Segment {
  readonly name: string;
  /** The share of the count, in parts of one (the shares of a function sum to 1). */
  readonly share: number;
  /** One input (`x`), or a pair (`x`, `y`) for the binary functions. */
  readonly make: (random: Random, index: number) => number | readonly [number, number];
}

function special(index: number): number {
  return SPECIALS[index % SPECIALS.length] ?? 0;
}

function trigSegments(): Segment[] {
  return [
    { name: 'random bits', share: 0.25, make: anyBits },
    { name: 'angles -4pi..4pi', share: 0.1, make: (r) => uniform(r, -2 * TWO_PI, 2 * TWO_PI) },
    {
      name: 'radial angles 2pi*i/n',
      share: 0.05,
      make: (r) => {
        const n = uniformInt(r, 1, 360);
        return (TWO_PI * uniformInt(r, 0, n)) / n;
      },
    },
    {
      name: 'multiples of pi/2 +- 8 ulp',
      share: 0.15,
      make: (r) => stepUlps(uniformInt(r, -(2 ** 20), 2 ** 20) * PI_OVER_TWO, uniformInt(r, -8, 8)),
    },
    { name: 'medium -2^20..2^20', share: 0.1, make: (r) => uniform(r, -(2 ** 20), 2 ** 20) },
    { name: 'large exponents', share: 0.15, make: (r) => withExponent(r, 0x413, 0x7fe) },
    { name: 'tiny exponents', share: 0.05, make: (r) => withExponent(r, 0, 0x3ec) },
    { name: 'thresholds', share: 0.1, make: (r) => nearThreshold(r, TRIG_THRESHOLDS) },
    { name: 'specials', share: 0.05, make: (_r, i) => special(i) },
  ];
}

function logSegments(): Segment[] {
  return [
    { name: 'random bits', share: 0.2, make: anyBits },
    {
      name: 'positive random bits',
      share: 0.15,
      make: (r) => fromWords(r() & 0x7fff_ffff, r()),
    },
    {
      name: 'near 1: 1 + k 2^-52',
      share: 0.1,
      make: (r) => 1 + uniformInt(r, -4096, 4096) * 2 ** -52,
    },
    { name: 'near 1: 1 +- 1e-3', share: 0.05, make: (r) => 1 + uniform(r, -1e-3, 1e-3) },
    { name: 'uniform 0..1e6', share: 0.1, make: (r) => uniform(r, 0, 1e6) },
    { name: 'integers 1..1e6', share: 0.05, make: (r) => uniformInt(r, 1, 1_000_000) },
    { name: 'subnormals', share: 0.05, make: (r) => fromWords(r() & 0x000f_ffff, r()) },
    { name: 'tiny normals', share: 0.05, make: (r) => withExponent(r, 1, 0x0ff) },
    { name: 'large exponents', share: 0.05, make: (r) => withExponent(r, 0x7e0, 0x7fe) },
    { name: 'thresholds', share: 0.1, make: (r) => nearThreshold(r, LOG_THRESHOLDS) },
    { name: 'specials', share: 0.1, make: (_r, i) => special(i) },
  ];
}

function log10Segments(): Segment[] {
  return [
    { name: 'random bits', share: 0.15, make: anyBits },
    {
      name: 'positive random bits',
      share: 0.15,
      make: (r) => fromWords(r() & 0x7fff_ffff, r()),
    },
    {
      name: 'powers of ten +- 8 ulp',
      share: 0.15,
      make: (r) => stepUlps(10 ** uniformInt(r, -30, 30), uniformInt(r, -8, 8)),
    },
    {
      name: 'tick steps: span / count',
      share: 0.1,
      make: (r) => uniform(r, 1e-3, 1e9) / uniformInt(r, 1, 20),
    },
    {
      name: 'near 1: 1 + k 2^-52',
      share: 0.1,
      make: (r) => 1 + uniformInt(r, -4096, 4096) * 2 ** -52,
    },
    { name: 'uniform 0..1e6', share: 0.1, make: (r) => uniform(r, 0, 1e6) },
    { name: 'subnormals', share: 0.05, make: (r) => fromWords(r() & 0x000f_ffff, r()) },
    { name: 'thresholds', share: 0.1, make: (r) => nearThreshold(r, LOG10_THRESHOLDS) },
    { name: 'specials', share: 0.1, make: (_r, i) => special(i) },
  ];
}

function atanSegments(): Segment[] {
  return [
    { name: 'random bits', share: 0.25, make: anyBits },
    { name: 'uniform -10..10', share: 0.15, make: (r) => uniform(r, -10, 10) },
    { name: 'uniform -1e4..1e4', share: 0.05, make: (r) => uniform(r, -1e4, 1e4) },
    { name: 'thresholds', share: 0.15, make: (r) => nearThreshold(r, ATAN_THRESHOLDS) },
    { name: 'tiny exponents', share: 0.05, make: (r) => withExponent(r, 0, 0x3e4) },
    { name: 'large exponents', share: 0.1, make: (r) => withExponent(r, 0x440, 0x7fe) },
    {
      name: 'near 1: 1 +- k 2^-53',
      share: 0.1,
      make: (r) => (r() & 1 ? 1 : -1) * (1 - uniformInt(r, 0, 4096) * 2 ** -53),
    },
    { name: 'specials', share: 0.15, make: (_r, i) => special(i) },
  ];
}

function inverseSegments(): Segment[] {
  return [
    { name: 'uniform -1..1', share: 0.3, make: (r) => uniform(r, -1, 1) },
    { name: 'random bits', share: 0.15, make: anyBits },
    {
      name: 'near +-1: 1 - k 2^-53',
      share: 0.15,
      make: (r) => (r() & 1 ? 1 : -1) * (1 - uniformInt(r, 0, 4096) * 2 ** -53),
    },
    { name: 'thresholds', share: 0.15, make: (r) => nearThreshold(r, INVERSE_THRESHOLDS) },
    { name: 'tiny exponents', share: 0.1, make: (r) => withExponent(r, 0, 0x3e4) },
    { name: 'outside -4..4', share: 0.05, make: (r) => uniform(r, -4, 4) },
    { name: 'specials', share: 0.1, make: (_r, i) => special(i) },
  ];
}

function atan2Segments(): Segment[] {
  return [
    { name: 'random bits', share: 0.2, make: (r) => [anyBits(r), anyBits(r)] },
    {
      name: 'uniform -1e4..1e4',
      share: 0.2,
      make: (r) => [uniform(r, -1e4, 1e4), uniform(r, -1e4, 1e4)],
    },
    {
      name: 'ratio 2^-100..2^100',
      share: 0.15,
      make: (r) => {
        const x = uniform(r, -1e3, 1e3);
        return [x * 2 ** uniformInt(r, -100, 100), x];
      },
    },
    { name: 'x = +-1', share: 0.05, make: (r) => [uniform(r, -10, 10), r() & 1 ? 1 : -1] },
    {
      name: 'similar exponents',
      share: 0.15,
      make: (r) => [withExponent(r, 0x3f0, 0x42f), withExponent(r, 0x3f0, 0x42f)],
    },
    {
      name: 'specials',
      share: 0.25,
      make: (r) => [special(uniformInt(r, 0, 1 << 20)), special(uniformInt(r, 0, 1 << 20))],
    },
  ];
}

function powSegments(): Segment[] {
  return [
    { name: 'random bits', share: 0.1, make: (r) => [anyBits(r), anyBits(r)] },
    {
      name: 'integer base 1..1e4, exponent -10..10',
      share: 0.15,
      make: (r) => [uniformInt(r, 1, 10_000), uniform(r, -10, 10)],
    },
    {
      name: 'cooling form, exponent 0..3',
      share: 0.1,
      make: (r) => [uniformInt(r, 1, 1000), uniform(r, 0, 3)],
    },
    { name: 'base in (0,1)', share: 0.1, make: (r) => [unit(r), uniform(r, -50, 50)] },
    {
      name: 'negative integer base',
      share: 0.1,
      make: (r) => [-uniformInt(r, 1, 1000), uniformInt(r, -20, 20)],
    },
    {
      name: 'exponent 2, 0.5, -1 and so on',
      share: 0.1,
      make: (r) => {
        const exponents = [2, 0.5, -1, -0.5, 3, 1 / 3, 2.5, -2];
        return [uniform(r, -100, 100), exponents[uniformInt(r, 0, exponents.length - 1)] ?? 2];
      },
    },
    { name: 'base 2, integer exponent', share: 0.05, make: (r) => [2, uniformInt(r, -1074, 1024)] },
    {
      name: 'similar exponents',
      share: 0.1,
      make: (r) => [withExponent(r, 0x3e0, 0x43f), withExponent(r, 0x3e0, 0x43f)],
    },
    {
      name: 'specials',
      share: 0.1,
      make: (r) => [special(uniformInt(r, 0, 1 << 20)), special(uniformInt(r, 0, 1 << 20))],
    },
  ];
}

function hypotSegments(): Segment[] {
  return [
    { name: 'random bits', share: 0.2, make: (r) => [anyBits(r), anyBits(r)] },
    {
      name: 'uniform -1e4..1e4',
      share: 0.25,
      make: (r) => [uniform(r, -1e4, 1e4), uniform(r, -1e4, 1e4)],
    },
    {
      name: 'similar exponents',
      share: 0.15,
      make: (r) => [withExponent(r, 0x3e0, 0x43f), withExponent(r, 0x3e0, 0x43f)],
    },
    {
      name: 'layout differences -500..500',
      share: 0.15,
      make: (r) => [uniform(r, -500, 500), uniform(r, -500, 500)],
    },
    {
      name: 'overflowing exponents',
      share: 0.05,
      make: (r) => [withExponent(r, 0x7e0, 0x7fe), withExponent(r, 0x7e0, 0x7fe)],
    },
    {
      name: 'specials',
      share: 0.2,
      make: (r) => [special(uniformInt(r, 0, 1 << 20)), special(uniformInt(r, 0, 1 << 20))],
    },
  ];
}

const PLANS: Record<FunctionName, Segment[]> = {
  sin: trigSegments(),
  cos: trigSegments(),
  tan: trigSegments(),
  asin: inverseSegments(),
  acos: inverseSegments(),
  atan: atanSegments(),
  log: logSegments(),
  log10: log10Segments(),
  atan2: atan2Segments(),
  pow: powSegments(),
  hypot: hypotSegments(),
};

function isBinary(name: FunctionName): name is BinaryName {
  return (BINARY as readonly string[]).includes(name);
}

interface SegmentRecord {
  readonly name: string;
  readonly start: number;
  readonly count: number;
}

/** The inputs of a function, and the segment each one came from. */
function buildInputs(
  name: FunctionName,
  total: number,
  seed: number
): { words: Float64Array; segments: SegmentRecord[] } {
  const binary = isBinary(name);
  const plan = PLANS[name];
  const random = makeRandom(seed);
  const perInput = binary ? 2 : 1;
  const words = new Float64Array(total * perInput);
  const segments: SegmentRecord[] = [];
  let start = 0;
  let index = 0;
  plan.forEach((segment, position) => {
    const count = position === plan.length - 1 ? total - start : Math.round(total * segment.share);
    for (let i = 0; i < count; i++, index++) {
      const value = segment.make(random, i);
      if (Array.isArray(value)) {
        words[index * 2] = value[0];
        words[index * 2 + 1] = value[1];
      } else {
        words[index] = value as number;
      }
    }
    segments.push({ name: segment.name, start, count });
    start += count;
  });
  return { words, segments };
}

/** The bits of a Float64Array, as little-endian `u64` words in a buffer. */
function toBytes(values: Float64Array): Buffer {
  if (endianness() !== 'LE') {
    throw new Error('the probe writes little-endian words and needs a little-endian host');
  }
  return Buffer.from(values.buffer, values.byteOffset, values.byteLength);
}

function canonicalBits(value: number): bigint {
  if (Number.isNaN(value)) {
    return 0x7ff8_0000_0000_0000n;
  }
  scratch.setFloat64(0, value, false);
  return scratch.getBigUint64(0, false);
}

function evaluateNode(name: FunctionName, words: Float64Array): Float64Array {
  const binary = isBinary(name);
  const math = Math as unknown as Record<string, (...args: number[]) => number>;
  const f = math[name];
  if (f === undefined) {
    throw new Error(`Math.${name} is missing`);
  }
  if (binary) {
    const out = new Float64Array(words.length / 2);
    for (let i = 0; i < out.length; i++) {
      out[i] = f(words[i * 2] ?? 0, words[i * 2 + 1] ?? 0);
    }
    return out;
  }
  const out = new Float64Array(words.length);
  for (let i = 0; i < out.length; i++) {
    out[i] = f(words[i] ?? 0);
  }
  return out;
}

/** The same results, computed by the page's own `Math` in Playwright's Chromium. */
async function evaluateChromium(
  name: FunctionName,
  words: Float64Array,
  page: Page
): Promise<Float64Array> {
  const binary = isBinary(name);
  const perCall = binary ? CHROME_CHUNK * 2 : CHROME_CHUNK;
  const out = new Float64Array(binary ? words.length / 2 : words.length);
  const inPage = (args: { name: string; base64: string; arity: number }): string => {
    const bytes = Uint8Array.from(atob(args.base64), (c) => c.charCodeAt(0));
    const view = new DataView(bytes.buffer);
    const f = (Math as unknown as Record<string, (...values: number[]) => number>)[args.name];
    if (f === undefined) {
      throw new Error(`Math.${args.name} is missing`);
    }
    const count = args.arity === 2 ? bytes.length / 16 : bytes.length / 8;
    const result = new DataView(new ArrayBuffer(count * 8));
    for (let i = 0; i < count; i++) {
      const value =
        args.arity === 2
          ? f(view.getFloat64(i * 16, true), view.getFloat64(i * 16 + 8, true))
          : f(view.getFloat64(i * 8, true));
      result.setFloat64(i * 8, value, true);
    }
    const outBytes = new Uint8Array(result.buffer);
    let text = '';
    for (const byte of outBytes) {
      text += String.fromCharCode(byte);
    }
    return btoa(text);
  };
  for (let start = 0; start < words.length; start += perCall) {
    const end = Math.min(words.length, start + perCall);
    const chunk = words.subarray(start, end);
    const base64 = Buffer.from(chunk.buffer, chunk.byteOffset, chunk.byteLength).toString('base64');
    const reply = await page.evaluate(inPage, { name, base64, arity: binary ? 2 : 1 });
    const bytes = Buffer.from(reply, 'base64');
    const values = new Float64Array(bytes.buffer, bytes.byteOffset, bytes.byteLength / 8);
    const outStart = binary ? start / 2 : start;
    out.set(values, outStart);
  }
  return out;
}

async function generate(dir: string, runtime: 'node' | 'chromium', total: number): Promise<void> {
  await mkdir(dir, { recursive: true });
  const names: FunctionName[] = [...UNARY, ...BINARY];
  const segmentsOut: Record<string, SegmentRecord[]> = {};
  let browser: Browser | undefined;
  let page: Page | undefined;
  let versionLine = `node ${process.version}`;
  if (runtime === 'chromium') {
    browser = await chromium.launch();
    page = await browser.newPage();
    await page.setContent('<!doctype html><title>jsmath probe</title>');
    versionLine = `chromium ${browser.version()}`;
  }
  try {
    for (const name of names) {
      const seed = 0x5eed_0000 + SEED_ORDER.indexOf(name);
      const { words, segments } = buildInputs(name, total, seed);
      segmentsOut[name] = segments;
      const values =
        page === undefined ? evaluateNode(name, words) : await evaluateChromium(name, words, page);
      const canonical = new Float64Array(values.length);
      for (const [i, value] of values.entries()) {
        const bits = canonicalBits(value);
        scratch.setBigUint64(0, bits, false);
        canonical[i] = scratch.getFloat64(0, false);
      }
      await writeFile(join(dir, `${name}.in`), toBytes(words));
      await writeFile(join(dir, `${name}.v8`), toBytes(canonical));
      console.log(`${name}: ${total} inputs, ${segments.length} segments (${versionLine})`);
    }
  } finally {
    await browser?.close();
  }
  await writeFile(
    join(dir, 'segments.json'),
    JSON.stringify({ runtime: versionLine, total, segments: segmentsOut }, null, 2)
  );
}

/** Reads a file of little-endian `u64` words into an aligned array of bit patterns. */
async function readWords(path: string): Promise<BigUint64Array> {
  const bytes = await readFile(path);
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return new BigUint64Array(copy.buffer);
}

function isNanBits(bits: bigint): boolean {
  scratch.setBigUint64(0, bits, false);
  return Number.isNaN(scratch.getFloat64(0, false));
}

function hex(bits: bigint): string {
  return `0x${bits.toString(16).padStart(16, '0')}`;
}

async function compare(dir: string, extension: 'rs' | 'node'): Promise<boolean> {
  const segmentsFile = JSON.parse(await readFile(join(dir, 'segments.json'), 'utf8')) as {
    runtime: string;
    total: number;
    segments: Record<string, SegmentRecord[]>;
  };
  console.log(`reference: ${segmentsFile.runtime}`);
  let clean = true;
  for (const name of [...UNARY, ...BINARY] as FunctionName[]) {
    const inputs = await readWords(join(dir, `${name}.in`));
    const expected = await readWords(join(dir, `${name}.v8`));
    const actual = await readWords(join(dir, `${name}.${extension}`));
    const binary = isBinary(name);
    const count = expected.length;
    if (actual.length !== count) {
      throw new Error(`${name}: ${actual.length} results for ${count} inputs`);
    }
    const segments = segmentsFile.segments[name] ?? [];
    let total = 0;
    let mismatches = 0;
    console.log(`\n${name}`);
    for (const segment of segments) {
      let segmentMismatches = 0;
      let first = '';
      for (let i = segment.start; i < segment.start + segment.count; i++) {
        total++;
        const want = expected[i] ?? 0n;
        const got = actual[i] ?? 0n;
        if (want === got || (isNanBits(want) && isNanBits(got))) {
          continue;
        }
        segmentMismatches++;
        if (first === '') {
          const x = inputs[binary ? i * 2 : i] ?? 0n;
          const y = binary ? (inputs[i * 2 + 1] ?? 0n) : null;
          first = `x=${hex(x)}${y === null ? '' : ` y=${hex(y)}`} v8=${hex(want)} rust=${hex(got)}`;
        }
      }
      mismatches += segmentMismatches;
      console.log(
        `  ${segment.name.padEnd(36)} ${String(segment.count).padStart(9)} inputs  ${String(segmentMismatches).padStart(6)} differ${first ? `  first: ${first}` : ''}`
      );
    }
    console.log(
      `  ${'total'.padEnd(36)} ${String(total).padStart(9)} inputs  ${String(mismatches).padStart(6)} differ`
    );
    if (mismatches > 0) {
      clean = false;
    }
  }
  return clean;
}

/** Writes the committed test data: an evenly spaced sample of each segment, with the V8 results. */
async function sample(dir: string, outDir: string, perSegment: number): Promise<void> {
  const segmentsFile = JSON.parse(await readFile(join(dir, 'segments.json'), 'utf8')) as {
    runtime: string;
    segments: Record<string, SegmentRecord[]>;
  };
  const header = `# ${segmentsFile.runtime} on ${process.platform} ${process.arch}, from the probe's inputs (see tools/port/jsmath-probe.ts)`;
  await mkdir(outDir, { recursive: true });
  for (const name of [...UNARY, ...BINARY] as FunctionName[]) {
    const inputs = await readWords(join(dir, `${name}.in`));
    const expected = await readWords(join(dir, `${name}.v8`));
    const binary = isBinary(name);
    const lines: string[] = [];
    for (const segment of segmentsFile.segments[name] ?? []) {
      const step = Math.max(1, Math.floor(segment.count / perSegment));
      for (let k = 0; k < segment.count && k / step < perSegment; k += step) {
        const i = segment.start + k;
        const x = hex(inputs[binary ? i * 2 : i] ?? 0n);
        const result = hex(expected[i] ?? 0n);
        lines.push(binary ? `${x} ${hex(inputs[i * 2 + 1] ?? 0n)} ${result}` : `${x} ${result}`);
      }
    }
    await writeFile(join(outDir, `${name}.txt`), `${[header, ...lines].join('\n')}\n`);
    console.log(`${name}: ${lines.length} cases`);
  }
}

function option(args: string[], flag: string): string | undefined {
  const index = args.indexOf(flag);
  return index === -1 ? undefined : args[index + 1];
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const command = args[0];
  if (command === 'generate' && args[1]) {
    const runtime = option(args, '--runtime') === 'chromium' ? 'chromium' : 'node';
    const total = Number(option(args, '--count') ?? DEFAULT_COUNT);
    await generate(args[1], runtime, total);
    return;
  }
  if (command === 'compare' && args[1]) {
    const clean = await compare(args[1], args.includes('--node') ? 'node' : 'rs');
    console.log(clean ? '\nall functions agree bit for bit' : '\nsome results differ');
    process.exitCode = clean ? 0 : 1;
    return;
  }
  if (command === 'sample' && args[1] && args[2]) {
    await sample(args[1], args[2], Number(option(args, '--per-segment') ?? 400));
    return;
  }
  console.error(
    'usage: jsmath-probe.ts generate <dir> [--runtime node|chromium] [--count N] | compare <dir> [--node] | sample <dir> <out-dir> [--per-segment K]'
  );
  process.exitCode = 2;
}

await main();
