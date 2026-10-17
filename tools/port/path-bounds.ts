/**
 * The corpus of `crates/ferrigura-path`: a few thousand SVG path data strings, with the `getBBox()` that
 * Chromium returns for each path element, read in the harness's browser (`withPage` of `geometry/common.ts`).
 *
 * The paths cover the arcs of every flag combination, rotations, tiny and huge radii, radii too small for
 * the end points, the commands the diagrams write (`M`, `H`, `V`, `Z`, relative and absolute, implicit
 * repeats), the smooth curves, and the cloud and bang outlines of the mindmap fixtures. The numbers are
 * written as `number()` writes them in the port (the shortest decimal that reads back as the double).
 *
 * Usage: pnpm port:path-bounds   (writes crates/ferrigura-path/tests/corpus/paths.json)
 */
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { root, withPage } from './geometry/common.js';

/** The number of paths of the corpus, before the fixed ones. */
const GENERATED = 3000;

/** A deterministic generator (mulberry32), so that the corpus is the same on every run. */
function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A number as the port writes it: the shortest decimal, with the sign and no exponent for sizes. */
function num(value: number): string {
  if (Math.abs(value) < 1e-10) return '0';
  return String(value);
}

/** A random number in [low, high), rounded to a decimal grid for half of the draws. */
function draw(rand: () => number, low: number, high: number): number {
  const value = low + (high - low) * rand();
  return rand() < 0.5 ? Math.round(value * 1000) / 1000 : value;
}

/** A random arc of a path: its radii, rotation, flags and end, relative or absolute. */
function arc(
  rand: () => number,
  relative: boolean,
  from: [number, number]
): { d: string; to: [number, number] } {
  const scale = [0.01, 0.5, 5, 50, 500, 50000][Math.floor(rand() * 6)];
  const rx = draw(rand, 0.001, 1) * scale;
  const ry = rand() < 0.3 ? rx : draw(rand, 0.001, 1) * scale;
  const rotation = draw(rand, -720, 720);
  const large = rand() < 0.5 ? 1 : 0;
  const sweep = rand() < 0.5 ? 1 : 0;
  const reach = scale * (rand() < 0.2 ? 0.05 : 3);
  const dx = draw(rand, -reach, reach);
  const dy = draw(rand, -reach, reach);
  const to: [number, number] = relative ? [from[0] + dx, from[1] + dy] : [dx, dy];
  const cmd = relative ? 'a' : 'A';
  const sep = rand() < 0.5 ? ',' : ' ';
  const body = `${num(rx)}${sep}${num(ry)} ${num(rotation)} ${large}${sep}${sweep} ${num(to[0] - (relative ? from[0] : 0))}${sep}${num(to[1] - (relative ? from[1] : 0))}`;
  return { d: `${cmd}${body}`, to };
}

/** The generated paths: each a move, then a few segments of the kinds above, then sometimes a close. */
function generated(rand: () => number): string[] {
  const out: string[] = [];
  for (let index = 0; index < GENERATED; index++) {
    let at: [number, number] = [draw(rand, -200, 200), draw(rand, -200, 200)];
    let d = `M${num(at[0])} ${num(at[1])}`;
    const count = 1 + Math.floor(rand() * 6);
    for (let step = 0; step < count; step++) {
      const kind = rand();
      const relative = rand() < 0.5;
      if (kind < 0.55) {
        const piece = arc(rand, relative, at);
        d += ` ${piece.d}`;
        at = piece.to;
      } else if (kind < 0.7) {
        const x = draw(rand, -200, 200);
        d += relative ? ` h${num(x)}` : ` H${num(x)}`;
        at = [relative ? at[0] + x : x, at[1]];
      } else if (kind < 0.8) {
        const y = draw(rand, -200, 200);
        d += relative ? ` v${num(y)}` : ` V${num(y)}`;
        at = [at[0], relative ? at[1] + y : y];
      } else if (kind < 0.9) {
        const x = draw(rand, -200, 200);
        const y = draw(rand, -200, 200);
        d += relative ? ` l${num(x)},${num(y)}` : ` L${num(x)} ${num(y)}`;
        at = relative ? [at[0] + x, at[1] + y] : [x, y];
      } else {
        const c = [
          draw(rand, -200, 200),
          draw(rand, -200, 200),
          draw(rand, -200, 200),
          draw(rand, -200, 200),
          draw(rand, -200, 200),
          draw(rand, -200, 200),
        ];
        d += relative ? ` c${c.map(num).join(' ')}` : ` C${c.map(num).join(' ')}`;
        at = relative ? [at[0] + c[4], at[1] + c[5]] : [c[4], c[5]];
      }
    }
    if (rand() < 0.3) d += ' Z';
    out.push(d);
  }
  return out;
}

/** The fixed paths: the outline of a cloud and of a bang, as the mindmap fixtures write them, and some
 * shapes of the grammar (implicit repeats, exponents, smooth curves, a path that stops at an error). */
const FIXED = [
  'M0 0 a14.0015625,14.0015625 0 0,1 23.3359375,-9.334375 a32.6703125,32.6703125 1 0,1 37.3375,-9.334375 a23.335937,23.335937 1 0,1 17.5,-4 H0 V0 Z',
  'M0 0 a17.68125,17.68125 1 0,0 29.46875,-5.7 a17.68125,17.68125 1 0,0 29.46875,0 a17.68125,17.68125 1 0,0 29.46875,5.7 H0 V0 Z',
  'M10 10 l 20 0 20 20 -20 0 z',
  'M0,0 10,0 20,10 30,0',
  'M0 0 1e2 1e1 5E-1 2 3e1 4 5 6 7 8 9',
  'M1 1 S5 5 10 10 T20 20 Q1 2 3 4',
  'M5 5 a0 10 0 0 1 10 10',
  'M5 5 a10 10 0 0 1 5 5 l 5 5 H 5 Z m 3 3 l 1 1',
  'M0 0 Z',
  'L1 1',
  'M0 0 a1 2 3 4 5 6 7',
];

/** The browser's `getBBox` of each path, as a float32 box. */
async function measure(
  paths: string[]
): Promise<{ x: number; y: number; width: number; height: number }[]> {
  return withPage('', (page) =>
    page.evaluate((list: string[]) => {
      const svgNs = 'http://www.w3.org/2000/svg';
      const svg = document.createElementNS(svgNs, 'svg');
      document.body.appendChild(svg);
      return list.map((d) => {
        const path = document.createElementNS(svgNs, 'path');
        path.setAttribute('d', d);
        svg.appendChild(path);
        const box = path.getBBox();
        svg.removeChild(path);
        return { x: box.x, y: box.y, width: box.width, height: box.height };
      });
    }, paths)
  );
}

async function main(): Promise<void> {
  const rand = generator(20261017);
  const paths = [...FIXED, ...generated(rand)];
  const boxes = await measure(paths);
  const corpus = paths.map((d, index) => ({ d, ...boxes[index] }));
  const target = join(root, 'crates/ferrigura-path/tests/corpus/paths.json');
  await writeFile(target, `${JSON.stringify(corpus, null, 1)}\n`);
  console.log(`wrote ${corpus.length} paths to ${target}`);
}

await main();
