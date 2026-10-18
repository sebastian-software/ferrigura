// Prints tests/data/js-sort.txt: Array.prototype.sort as V8 in Chromium runs it, on arrays of element ids with
// comparators over per-element keys. The cases are drawn and sorted inside Chromium, Playwright's (151), the
// browser of the swimlane reference:
//
//   pnpm exec tsx crates/ferrigura-swimlane/tools/js-sort.ts > crates/ferrigura-swimlane/tests/data/js-sort.txt
//
// Node is not the oracle: V8 13.6 (Node 24) makes other comparisons than Chromium 151 below 16 elements and from
// 64 on, so its call sequences differ, and for the inconsistent comparators its permutations too.
//
// Up to 16 elements Chromium's comparisons depend on the JIT state of the code that calls sort: the first call
// of a function, a call site that has run some thousands of sorts, and code made by new Function each make other
// comparisons. Each case sorts in code of its own, made by new Function (sortAtFreshSite), so that no case
// depends on the cases before it. From 17 elements on, every kind of call site sorted alike in the probes.
//
// Each case sorts the ids 0..n-1, where element i has the key keys[i], with one comparator. The data holds the
// final permutation of the ids and a hash of the sequence of comparator calls (the arguments a, b of each call,
// in order). For n <= 10 the full sequence of calls is written too.
//
// File format: lines starting with '#' are comments; fields are separated by one space; '-' stands for an
// empty field.
//   case <comparator> <keys> <n> <seed> <calls> <hash> <permutation>
//     <permutation>: the ids in sorted order, separated by commas.
//     <calls>: the number of comparator calls. <hash>: 16 hex digits, see callHash.
//   log <comparator> <keys> <n> <seed> <a:b>...: every call of a case with n <= 10, as the arguments a and b.
//
// Comparators, with a and b the element ids as V8 passes them to the comparator:
//   diff       keys[a] - keys[b]
//   heuristic  +1 or -1 by the ids (-1 when a < b) if a key is NaN, keys[a] - keys[b] otherwise
//   hash       -1, 0 or +1 from pairHash(seed, a, b): an inconsistent order, with ties
// Key sets, drawn from mulberry32(seed) in the order given here:
//   special    one of NaN, 0, -0, Infinity, -Infinity, 1, 2, 3, -1, 0.5
//   doubles    (u - 0.5) * 2000 for a uniform u
//   ties       NaN if u < 1/8, else an integer from -4 to 4
//   runs       ascending and descending runs of integers, with ties and NaN (1/16): galloping and descents
// Lengths: 0 to 300, then 500, 1000 and 3000. The seed of a case depends on its length and its combination.
//
// callHash: two 32-bit lanes over the code a * 65536 + b of each call (a, b < 65536). The first lane is FNV-1a
// from 0x811c9dc5 with prime 0x01000193. The second starts at 0x6a09e667, multiplies by 0x9e3779b1 and folds the
// high half down (h ^= h >>> 16). Each lane is written as 8 hex digits, the first lane first.
//
// The random draws are mulberry32 and Math.imul arithmetic only (no Math functions that differ between
// engines), so the output is the same on every run of the same browser.

import { chromium } from 'playwright';

// The whole generator, run in the browser. Playwright sends the function's source to the page, so it uses no name
// from outside its body.
function generate(browserVersion: string): string[] {
  const SPECIAL: number[] = [NaN, 0, -0, Infinity, -Infinity, 1, 2, 3, -1, 0.5];
  const KINDS = ['diff', 'heuristic', 'hash'] as const;
  const KEY_SETS = ['special', 'doubles', 'ties', 'runs'] as const;
  type Kind = (typeof KINDS)[number];
  type KeySet = (typeof KEY_SETS)[number];
  type Call = [number, number];

  const mulberry32 = (seed: number): (() => number) => {
    let a = seed >>> 0;
    return (): number => {
      a = (a + 0x6d2b79f5) | 0;
      let t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  };

  // One mulberry32 step of the seed, the ids and the constants: an unsigned 32-bit number.
  const pairHash = (seed: number, a: number, b: number): number => {
    let t = seed ^ Math.imul(a, 0x9e3779b1) ^ Math.imul(b, 0x85ebca77);
    t = (t + 0x6d2b79f5) | 0;
    t = Math.imul(t ^ (t >>> 15), 1 | t);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return (t ^ (t >>> 14)) >>> 0;
  };

  const keysOf = (keySet: KeySet, n: number, seed: number): number[] => {
    const rand = mulberry32(seed);
    const keys: number[] = [];
    if (keySet === 'runs') {
      let value = Math.floor((rand() - 0.5) * 2000);
      let descending = false;
      let remaining = 0;
      for (let i = 0; i < n; i++) {
        if (remaining === 0) {
          remaining = 1 + Math.floor(rand() * 200);
          descending = rand() < 0.25;
        }
        remaining--;
        if (rand() < 1 / 16) {
          keys.push(NaN);
        } else {
          const step = Math.floor(rand() * 3);
          value = descending ? value - step : value + step;
          keys.push(value);
        }
      }
      return keys;
    }
    for (let i = 0; i < n; i++) {
      if (keySet === 'special') {
        keys.push(SPECIAL[Math.floor(rand() * SPECIAL.length)]);
      } else if (keySet === 'doubles') {
        keys.push((rand() - 0.5) * 2000);
      } else {
        keys.push(rand() < 1 / 8 ? NaN : Math.floor(rand() * 9) - 4);
      }
    }
    return keys;
  };

  const comparatorOf = (
    kind: Kind,
    keys: number[],
    seed: number
  ): ((a: number, b: number) => number) => {
    if (kind === 'diff') {
      return (a, b) => keys[a] - keys[b];
    }
    if (kind === 'heuristic') {
      return (a, b) => {
        if (Number.isNaN(keys[a]) || Number.isNaN(keys[b])) {
          return a < b ? -1 : 1;
        }
        return keys[a] - keys[b];
      };
    }
    return (a, b) => (pairHash(seed, a, b) % 3) - 1;
  };

  const callHash = (calls: Call[]): string => {
    let h1 = 0x811c9dc5 | 0;
    let h2 = 0x6a09e667 | 0;
    for (const [a, b] of calls) {
      const code = a * 65536 + b;
      h1 = Math.imul(h1 ^ code, 0x01000193);
      h2 = Math.imul(h2 ^ code, 0x9e3779b1);
      h2 ^= h2 >>> 16;
    }
    return (h1 >>> 0).toString(16).padStart(8, '0') + (h2 >>> 0).toString(16).padStart(8, '0');
  };

  // Sorts the ids 0..n-1 in code of its own, made by new Function, and returns the permutation and the calls.
  // Up to 16 elements the comparisons depend on the JIT state of the calling code, and new code has none from the
  // cases before.
  const sortAtFreshSite = (
    n: number,
    compare: (a: number, b: number) => number
  ): [number[], Call[]] => {
    const body = [
      'const calls = [];',
      'const ids = Array.from({ length: n }, (_, i) => i);',
      'ids.sort((a, b) => { calls.push([a, b]); return compare(a, b); });',
      'return [ids, calls];',
    ].join('\n');
    // eslint-disable-next-line @typescript-eslint/no-implied-eval -- each case needs code of its own (see above)
    const sortOnce = new Function('n', 'compare', body) as (
      n: number,
      compare: (a: number, b: number) => number
    ) => [number[], Call[]];
    return sortOnce(n, compare);
  };

  const lengths: number[] = [...Array.from({ length: 301 }, (_, i) => i), 500, 1000, 3000];
  const out: string[] = [
    `# Array.prototype.sort (V8 in Chromium ${browserVersion}) on the ids 0..n-1 with a comparator over per-element keys.`,
    '# case <comparator> <keys> <n> <seed> <calls> <hash> <permutation>; log <comparator> <keys> <n> <seed> <a:b>...',
    '# Generated by: pnpm exec tsx crates/ferrigura-swimlane/tools/js-sort.ts (Playwright 1.62.1).',
  ];
  let combination = 0;
  for (const kind of KINDS) {
    for (const keySet of KEY_SETS) {
      combination++;
      for (const n of lengths) {
        const seed = (Math.imul(n, 0x9e3779b1) ^ Math.imul(combination, 0x85ebca77)) >>> 0;
        const keys = keysOf(keySet, n, seed);
        const compare = comparatorOf(kind, keys, seed);
        const [ids, calls] = sortAtFreshSite(n, compare);
        const permutation = ids.length > 0 ? ids.join(',') : '-';
        out.push(
          `case ${kind} ${keySet} ${n} ${seed} ${calls.length} ${callHash(calls)} ${permutation}`
        );
        if (n <= 10) {
          const pairs = calls.map(([a, b]) => `${a}:${b}`);
          const fields = ['log', kind, keySet, String(n), String(seed), ...pairs];
          out.push(fields.join(' '));
        }
      }
    }
  }
  return out;
}

async function main(): Promise<void> {
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    // tsx (esbuild) may wrap functions in __name(...) to keep their names; the page needs a stand-in for it.
    await page.evaluate('globalThis.__name = (target) => target;');
    const lines = await page.evaluate(generate, browser.version());
    process.stdout.write(lines.join('\n') + '\n');
  } finally {
    await browser.close();
  }
}

main().catch((error: unknown) => {
  process.stderr.write(`${String(error)}\n`);
  process.exit(1);
});
