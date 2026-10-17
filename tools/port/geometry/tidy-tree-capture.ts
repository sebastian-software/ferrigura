/**
 * Captures the reference's tidy tree layout (`layout: tidy-tree`) in Chromium, for the port's test
 * `crates/ferrigura-render/tests/tidy_tree_reference.rs`:
 *
 * - every fixture of `e2e/diagrams/mindmap` that asks for the layout, rendered by the reference with
 *   the removed package registered. Its input is what the renderer hands the layout (the measured
 *   box of each node, `render.ts` 93-105), and its output is the layout's result;
 * - hand-picked trees and seeded random trees (mulberry32), laid out in the page by the package's
 *   `executeTidyTreeLayout`. The random trees have depth up to 4, up to 8 children, a few zero sizes
 *   and every mindmap shape.
 *
 * The package and its library come from `tidy-tree-bundle.ts`. The output is written to
 * `crates/ferrigura-render/tests/data/tidy-tree/` (`fixtures.json` and `generated.json`), with only
 * the fields the port compares: the node and edge fields of the reference's result, without the
 * `originalNode` of each node (which is the input node).
 *
 * Usage: pnpm port:tidy-tree
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import {
  decodeFixtureEntities,
  effectiveSettings,
  mmdFiles,
  patchText,
  renderReference,
  root,
  withPage,
  type JsonValue,
} from './common.js';
import { ensureTidyTreeBundle, TIDY_TREE_URL } from './tidy-tree-bundle.js';

const fixtureRoot = join(root, 'e2e/diagrams/mindmap');
/**
 * The committed corpus. `TIDY_TREE_OUT` writes elsewhere, and `TIDY_TREE_RANDOM` sets the number of
 * random trees: the port's ignored test reads a larger run from `target/` (see its documentation).
 */
const outputDir =
  process.env.TIDY_TREE_OUT ?? join(root, 'crates/ferrigura-render/tests/data/tidy-tree');

/** The reference's input node and edge, as the layout reads them. */
interface TidyNode {
  id: string;
  width: number;
  height: number;
  shape: string;
}
interface TidyEdge {
  id: string;
  start: string;
  end: string;
}
interface TidyInput {
  nodes: TidyNode[];
  edges: TidyEdge[];
}

/** The reference's result, projected to the fields the port compares. */
interface TidyOutput {
  nodes: Record<string, JsonValue>[];
  edges: Record<string, JsonValue>[];
}

/** A captured case: an input and the result, or the error the reference rejected with. */
interface TidyCase {
  name: string;
  input: TidyInput;
  output?: TidyOutput;
  error?: string;
}

/** The fields of the reference's result that the port compares. */
const NODE_FIELDS = ['id', 'x', 'y', 'section', 'width', 'height'] as const;
const EDGE_FIELDS = [
  'id',
  'source',
  'target',
  'startX',
  'startY',
  'midX',
  'midY',
  'endX',
  'endY',
  'points',
  'sourceSection',
  'targetSection',
  'sourceWidth',
  'sourceHeight',
  'targetWidth',
  'targetHeight',
] as const;

/** The reference's result as the port's fields; `JSON.stringify` drops the undefined ones. */
function projectOutput(raw: unknown): TidyOutput {
  const result = raw as {
    nodes: Record<string, unknown>[];
    edges: Record<string, unknown>[];
  };
  const pick = (record: Record<string, unknown>, fields: readonly string[]) =>
    Object.fromEntries(
      fields.map((field) => [field, record[field] ?? null]).filter(([, value]) => value !== null)
    ) as Record<string, JsonValue>;
  return {
    nodes: result.nodes.map((node) => pick(node, NODE_FIELDS)),
    edges: result.edges.map((edge) => pick(edge, EDGE_FIELDS)),
  };
}

/** The seeded generator of the random trees (the same mulberry32 as the reference harness). */
function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** The shapes of the mindmap DB (`mindmapDb.ts` 248-270), with the round ones among them. */
const SHAPES = [
  'rect',
  'rounded',
  'squareRect',
  'roundedRect',
  'circle',
  'cloud',
  'bang',
  'mindmapCircle',
  'defaultMindmapNode',
  'hexagon',
];

/**
 * A random mindmap-shaped tree: pre-order ids, the edges of each parent in the order of its children
 * (as `generateEdges` gives them), and about one node in twenty with a zero width or height.
 */
function randomTree(
  random: () => number,
  maxNodes: number,
  maxDepth: number,
  maxChildren: number
): TidyInput {
  const nodes: TidyNode[] = [];
  const childrenOf = new Map<string, string[]>();
  const make = (): string => {
    const id = String(nodes.length);
    nodes.push({
      id,
      width: random() < 0.05 ? 0 : 20 + random() * 220,
      height: random() < 0.05 ? 0 : 16 + random() * 60,
      shape: SHAPES[Math.floor(random() * SHAPES.length)],
    });
    return id;
  };
  const grow = (id: string, depth: number): void => {
    if (depth >= maxDepth) return;
    const count = random() < 0.35 ? 0 : 1 + Math.floor(random() * maxChildren);
    const children: string[] = [];
    for (let k = 0; k < count && nodes.length < maxNodes; k++) {
      const child = make();
      children.push(child);
      grow(child, depth + 1);
    }
    childrenOf.set(id, children);
  };
  grow(make(), 1);
  const edges: TidyEdge[] = [];
  for (const node of nodes) {
    for (const child of childrenOf.get(node.id) ?? []) {
      edges.push({ id: `edge_${node.id}_${child}`, start: node.id, end: child });
    }
  }
  return { nodes, edges };
}

/** Hand-picked trees: the probes of NOTES 3.6 and the edge cases of the reference's code. */
function handPicked(): { name: string; input: TidyInput }[] {
  const node = (id: string, width: number, height: number, shape = 'rect'): TidyNode => ({
    id,
    width,
    height,
    shape,
  });
  const edge = (start: string, end: string): TidyEdge => ({
    id: `edge_${start}_${end}`,
    start,
    end,
  });
  return [
    { name: 'single-node', input: { nodes: [node('0', 120, 40)], edges: [] } },
    {
      name: 'root-with-one-child-on-the-left',
      input: { nodes: [node('0', 120, 40), node('1', 60, 30)], edges: [edge('0', '1')] },
    },
    {
      name: 'root-with-two-leaves-of-height-30',
      input: {
        nodes: [node('0', 100, 50), node('1', 60, 30), node('2', 60, 30)],
        edges: [edge('0', '1'), edge('0', '2')],
      },
    },
    {
      name: 'same-side-edge-hooks-back',
      input: {
        nodes: [node('0', 100, 50), node('1', 60, 30), node('2', 50, 20)],
        edges: [edge('0', '1'), edge('1', '2')],
      },
    },
    {
      name: 'circle-root-is-clipped-as-a-rectangle',
      input: {
        nodes: [
          node('0', 134.97, 51, 'mindmapCircle'),
          node('1', 60, 30, 'circle'),
          node('2', 60, 30, 'cloud'),
          node('3', 60, 30, 'bang'),
        ],
        edges: [edge('0', '1'), edge('0', '2'), edge('0', '3')],
      },
    },
    {
      name: 'children-out-of-the-node-list-order',
      input: {
        nodes: [node('2', 40, 20), node('0', 100, 50), node('1', 40, 20), node('3', 40, 20)],
        edges: [edge('0', '1'), edge('0', '2'), edge('0', '3')],
      },
    },
    {
      name: 'child-missing-from-the-nodes',
      input: {
        nodes: [node('0', 100, 50), node('1', 40, 20), node('3', 40, 20)],
        edges: [edge('0', '1'), edge('0', '2'), edge('0', '3')],
      },
    },
    {
      name: 'zero-sized-nodes',
      input: {
        nodes: [node('0', 0, 0), node('1', 0, 30), node('2', 60, 0)],
        edges: [edge('0', '1'), edge('0', '2')],
      },
    },
    {
      name: 'cycle',
      input: {
        nodes: [node('0', 10, 10), node('1', 10, 10)],
        edges: [edge('0', '1'), edge('1', '0')],
      },
    },
    { name: 'no-nodes', input: { nodes: [], edges: [] } },
  ];
}

/** Random trees: the shapes of their sizes vary with the seed, and some list the root last. */
const RANDOM_TREES = Number(process.env.TIDY_TREE_RANDOM ?? 240);
const RANDOM_SEED = 20261017;

function randomCases(): { name: string; input: TidyInput }[] {
  const cases: { name: string; input: TidyInput }[] = [];
  for (let index = 0; index < RANDOM_TREES; index++) {
    const random = mulberry32(RANDOM_SEED + index);
    // Most trees are of 6 to 36 nodes, so that subtrees of unequal depth (the threads) are common.
    const input = randomTree(
      random,
      6 + Math.floor(random() * 30),
      2 + Math.floor(random() * 3),
      2 + Math.floor(random() * 6)
    );
    // Every seventh tree lists its nodes in reverse, so the root is not the first node.
    if (index % 7 === 6) input.nodes.reverse();
    cases.push({ name: `random-${String(index).padStart(3, '0')}`, input });
  }
  return cases;
}

/** The fixtures of `e2e/diagrams/mindmap` that set `layout: tidy-tree`, as paths under the folder. */
async function tidyFixtures(): Promise<string[]> {
  const found: string[] = [];
  for (const file of await mmdFiles(fixtureRoot)) {
    const source = await readFile(join(fixtureRoot, file), 'utf8');
    if (effectiveSettings(source).layout === 'tidy-tree') found.push(file);
  }
  return found;
}

async function main(): Promise<void> {
  await ensureTidyTreeBundle();
  const fixtures: TidyCase[] = [];
  const generated: TidyCase[] = [];
  const clock = { now: 1_000_000_000_000, seed: 1 };

  await withPage('const measure = () => null;', async (page) => {
    for (const file of await tidyFixtures()) {
      const source = await decodeFixtureEntities(
        page,
        await readFile(join(fixtureRoot, file), 'utf8')
      );
      const reference = await renderReference(
        page,
        `tidy-${fixtures.length}`,
        patchText(source),
        clock
      );
      if (!reference.ok || reference.svg === null) {
        throw new Error(`${file}: the reference failed: ${reference.error}`);
      }
      // The harness check: a fixture that asks for the tidy tree must reach the removed package's
      // layout, once per render. A fallback would otherwise be captured as if it were the tree.
      if (reference.tidyTree.length !== 1) {
        throw new Error(`${file}: ${reference.tidyTree.length} tidy tree layouts, expected 1`);
      }
      const [call] = reference.tidyTree;
      fixtures.push({
        name: file,
        input: call.input as unknown as TidyInput,
        output: projectOutput(call.output),
      });
      console.log(`✓ ${file}: ${(call.input as unknown as TidyInput).nodes.length} nodes`);
    }

    const cases = [...handPicked(), ...randomCases()];
    const results = (await page.evaluate(
      async ([url, inputs]) => {
        await import(url);
        const tidy = (
          window as unknown as {
            __tidyTree: { executeTidyTreeLayout: (data: unknown) => Promise<unknown> };
          }
        ).__tidyTree;
        const out: ({ output: unknown } | { error: string })[] = [];
        for (const input of inputs) {
          try {
            out.push({ output: await tidy.executeTidyTreeLayout(input) });
          } catch (error) {
            out.push({ error: error instanceof Error ? error.message : String(error) });
          }
        }
        return out;
      },
      [TIDY_TREE_URL, cases.map((testCase) => testCase.input)] as const
    )) as ({ output: unknown } | { error: string })[];

    cases.forEach((testCase, index) => {
      const result = results[index];
      const captured: TidyCase = { name: testCase.name, input: testCase.input };
      if ('error' in result) captured.error = result.error;
      else captured.output = projectOutput(result.output);
      generated.push(captured);
    });
  });

  // One line of JSON per file: the corpus is committed, so it stays compact.
  const source =
    '@mermaid-js/layout-tidy-tree 1.0.1 at 2d85261d6^, run in Chromium by pnpm port:tidy-tree';
  await mkdir(outputDir, { recursive: true });
  await writeFile(
    join(outputDir, 'fixtures.json'),
    JSON.stringify({ source, cases: fixtures }) + '\n'
  );
  await writeFile(
    join(outputDir, 'generated.json'),
    JSON.stringify({ source, cases: generated }) + '\n'
  );
  const errors = generated.filter((testCase) => testCase.error !== undefined).length;
  console.log(
    `captured ${fixtures.length} fixtures and ${generated.length} generated cases (${errors} rejected by the reference)`
  );
}

await main();
