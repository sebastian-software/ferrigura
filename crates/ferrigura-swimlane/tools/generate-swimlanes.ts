/* eslint-disable no-console -- CLI tool: the written files are reported on the console */
/**
 * Seeded generator of `swimlane-beta` diagrams for the reference corpus. The generator is
 * deterministic: the same seed and count give the same text. `capture-corpus.ts` imports it; run on
 * its own it writes the diagrams as `target/swimlane-generated/generated-NNN.mmd`:
 *
 *   pnpm exec tsx crates/ferrigura-swimlane/tools/generate-swimlanes.ts [seed] [count]
 *
 * Each diagram has 1 to 5 lanes (`subgraph` with a title, top level only), 1 to 6 nodes per lane, 1 or
 * 2 loose nodes outside any lane (the synthesised default lane), edges within and across lanes, back
 * edges (cycles), edge labels in both forms, all six node shapes, and ids with a hyphen and with an
 * underscore. The direction is one of the four the grammar accepts (`LR`, `TB`, `RL`, `BT`).
 * Node ids start with `n` or `loose` and are ASCII, so the grammar reads them as nodes.
 */
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** The seed of the reference corpus (the generated diagrams of `capture-corpus.ts`). */
export const GENERATOR_SEED = 20261018;
/** The number of generated diagrams in the corpus. */
export const GENERATED_COUNT = 60;

export interface GeneratedSwimlane {
  /** Position in the sequence, from 0. */
  index: number;
  text: string;
  nodes: number;
  edges: number;
  lanes: number;
  direction: string;
}

/** mulberry32: a seeded 32-bit generator returning numbers in [0, 1). */
export function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const DIRECTIONS = ['LR', 'TB', 'LR', 'TB', 'RL', 'BT'];
const SHAPES: readonly ((label: string) => string)[] = [
  (label) => `[${label}]`,
  (label) => `(${label})`,
  (label) => `{${label}}`,
  (label) => `([${label}])`,
  (label) => `[(${label})]`,
  (label) => `{{${label}}}`,
];
const LABELS = [
  'Receive request',
  'Check stock',
  'Approve',
  'Prepare kit',
  'Notify client',
  'Review file',
  'Archive copy',
  'Escalate',
  'Close ticket',
  'Send invoice',
  'Plan visit',
  'Log outcome',
  'Confirm order',
  'Pack goods',
  'Ship parcel',
  'Update record',
];
const EDGE_LABELS = ['Yes', 'No', 'Retry', 'Agree', 'Later'];

/** The generated diagrams of one seed, in order. */
export function generateSwimlanes(
  seed: number = GENERATOR_SEED,
  count: number = GENERATED_COUNT
): GeneratedSwimlane[] {
  const random = mulberry32(seed);
  const pick = <T>(items: readonly T[]): T => items[Math.floor(random() * items.length)];
  const between = (low: number, high: number): number =>
    low + Math.floor(random() * (high - low + 1));
  const diagrams: GeneratedSwimlane[] = [];
  for (let index = 0; index < count; index++) {
    const direction = pick(DIRECTIONS);
    const laneCount = between(1, 5);
    const lines = [`swimlane-beta ${direction}`];
    const ids: string[] = [];
    for (let lane = 1; lane <= laneCount; lane++) {
      lines.push(`  subgraph L${lane} [Team ${lane}]`);
      const nodeCount = between(1, 6);
      for (let j = 1; j <= nodeCount; j++) {
        // Odd lanes use a hyphen in the ids and even lanes an underscore.
        const id = lane % 2 === 1 ? `n${lane}-${j}` : `n${lane}_${j}`;
        lines.push(`    ${id}${pick(SHAPES)(pick(LABELS))}`);
        ids.push(id);
      }
      lines.push('  end');
    }
    const looseCount = between(1, 2);
    for (let i = 1; i <= looseCount; i++) {
      const id = `loose_${i}`;
      lines.push(`  ${id}${pick(SHAPES)(pick(LABELS))}`);
      ids.push(id);
    }

    // Edges between distinct nodes, one per ordered pair, then one forced back edge when the random
    // draw made none, so that every diagram has a cycle to remove.
    const edgeCount = ids.length + between(0, Math.floor(ids.length / 2));
    const seen = new Set<string>();
    const edgeLines: string[] = [];
    const addEdge = (from: string, to: string): void => {
      const key = `${from}>${to}`;
      if (from === to || seen.has(key)) {
        return;
      }
      seen.add(key);
      const style = between(0, 2);
      if (random() < 0.3) {
        const label = pick(EDGE_LABELS);
        edgeLines.push(
          style === 0
            ? `  ${from} -- ${label} --> ${to}`
            : style === 1
              ? `  ${from} -->|${label}| ${to}`
              : `  ${from} --> ${to}`
        );
      } else {
        edgeLines.push(`  ${from} --> ${to}`);
      }
    };
    for (let e = 0; e < edgeCount; e++) {
      addEdge(pick(ids), pick(ids));
    }
    const position = new Map(ids.map((id, i) => [id, i]));
    const hasBackEdge = [...seen].some((key) => {
      const [from, to] = key.split('>');
      return (position.get(from) ?? 0) > (position.get(to) ?? 0);
    });
    if (!hasBackEdge && ids.length > 1) {
      addEdge(ids[ids.length - 1], ids[0]);
    }
    lines.push(...edgeLines);
    diagrams.push({
      index,
      text: `${lines.join('\n')}\n`,
      nodes: ids.length,
      edges: edgeLines.length,
      lanes: laneCount,
      direction,
    });
  }
  return diagrams;
}

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

function main(): void {
  const seed = Number.parseInt(process.argv[2] ?? `${GENERATOR_SEED}`, 10);
  const count = Number.parseInt(process.argv[3] ?? `${GENERATED_COUNT}`, 10);
  const outDir = join(REPO_ROOT, 'target/swimlane-generated');
  mkdirSync(outDir, { recursive: true });
  const diagrams = generateSwimlanes(seed, count);
  for (const diagram of diagrams) {
    writeFileSync(
      join(outDir, `generated-${String(diagram.index).padStart(3, '0')}.mmd`),
      diagram.text
    );
  }
  console.log(`Wrote ${diagrams.length} diagrams (seed ${seed}) to ${outDir}`);
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
