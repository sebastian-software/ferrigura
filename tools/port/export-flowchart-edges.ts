/**
 * Flowchart edge paths, from the reference: the `d` that `insertEdge` of `edges.js` writes for a
 * list of points, and the line-hop rewrites of `lineJump.ts`. The ferrigura renderer compares its
 * paths against this fixture, so every `d` comes from the reference's own functions and none of
 * their private helpers is copied here.
 *
 * `insertEdge` runs in jsdom. The jsdom globals are installed before any reference module is
 * evaluated, because the reference touches `document` while it loads. The function is called the
 * way the ELK path calls it: `skipIntersect` is on, so the nodes are not clipped and the start and
 * end nodes are not used. The hop functions are DOM-free and are called directly.
 *
 * d3 is loaded from the reference's own dependency, so the selection that `insertEdge` receives
 * is the one `edges.js` uses. The repository root has no d3 of its own.
 *
 * The inputs are fixed in this file: hand-picked cases, then cases from a PRNG with a fixed seed.
 * Nothing reads the clock or the environment, so the output is the same bytes on every run.
 *
 * Usage: `pnpm run port:edge-paths`, which writes
 * `crates/ferrigura-render/tests/fixtures/flowchart-edge-paths.json`.
 */
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { JSDOM } from 'jsdom';

import { openReference } from './reference.ts';

const root = fileURLToPath(new URL('../../', import.meta.url));
const outputPath = join(root, 'crates/ferrigura-render/tests/fixtures/flowchart-edge-paths.json');

const SVG_NS = 'http://www.w3.org/2000/svg';
const D3_MODULE = '/packages/mermaid/node_modules/d3/src/index.js';
const EDGES_MODULE = '/packages/mermaid/src/rendering-util/rendering-elements/edges.js';
const LINE_JUMP_MODULE = '/packages/mermaid/src/rendering-util/rendering-elements/lineJump.ts';

/** Seed of the PRNG that generates the last path cases. Fixed, so the inputs never change. */
const GENERATOR_SEED = 0x6d65726d;
const GENERATED_PATHS = 6;
const JUMP_RADIUS = 6;
const JUMP_STYLES = ['arc', 'gap'] as const;
const HOP_EDGE_IDS = ['a', 'b', 'c', 'd'];

interface Point {
  x: number;
  y: number;
}

/** The fields that a path case and a hop edge have in common. */
interface EdgeCase {
  curve: string;
  arrowTypeStart: string;
  arrowTypeEnd: string;
  points: Point[];
}

interface PathRecord extends EdgeCase {
  id: string;
  d: string;
}

/** The fields of the edge object that `insertEdge` reads. */
interface InsertEdgeInput extends EdgeCase {
  id: string;
  look: 'classic';
  thickness: 'normal';
  pattern: 'solid';
  classes: 'flowchart-link';
  style: string[];
  cssCompiledStyles: string[];
  arrowheadStyle: string;
}

interface D3Module {
  select(node: Element): unknown;
}

interface EdgesModule {
  insertEdge(
    elem: unknown,
    edge: InsertEdgeInput,
    clusterDb: Map<string, unknown>,
    diagramType: string,
    startNode: Record<string, unknown>,
    endNode: Record<string, unknown>,
    diagramId: string,
    skipIntersect: boolean
  ): unknown;
}

interface EdgeGeom {
  id: string;
  points: Point[];
  curve?: string;
  arrowTypeStart?: string;
  arrowTypeEnd?: string;
}

interface Crossing {
  jumpEdgeId: string;
  otherEdgeId: string;
  segIndex: number;
  t: number;
  point: Point;
}

interface LineJumpConfig {
  enabled: boolean;
  jumpRadius: number;
  jumpStyle: 'arc' | 'gap';
}

interface LineJumpModule {
  findEdgeIntersections(edges: EdgeGeom[]): Crossing[];
  processEdgesWithJumps(edges: EdgeGeom[], config: LineJumpConfig): Map<string, string>;
}

interface HopEdgeRecord extends EdgeCase {
  id: string;
}

interface LineHopRecord {
  id: string;
  jumpStyle: 'arc' | 'gap';
  edges: HopEdgeRecord[];
  expected: Record<string, string>;
}

/** A case as this file writes it: the points as [x, y] pairs. */
function edgeCase(
  curve: string,
  arrowTypeStart: string,
  arrowTypeEnd: string,
  points: [number, number][]
): EdgeCase {
  return { curve, arrowTypeStart, arrowTypeEnd, points: points.map(([x, y]) => ({ x, y })) };
}

/**
 * Hand-picked path cases. They cover every curve of the reference's `insertEdge`, two and more
 * points, bends in both orientations, reversed directions, segments of 0.1 to 8 px, collinear and
 * repeated points, the arrow pairs with the marker offsets, diagonal segments, and coordinates
 * with decimals.
 */
const HAND_PICKED: EdgeCase[] = [
  // Two points.
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [60, 0],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [10, 80],
    [10, 20],
  ]),
  edgeCase('basis', 'none', 'arrow_cross', [
    [100, 10],
    [20, 10],
  ]),
  edgeCase('cardinal', 'none', 'arrow_circle', [
    [0, 0],
    [30, 40],
  ]),
  edgeCase('natural', 'none', 'arrow_point', [
    [0, 0.3333333],
    [0, 100.123456],
  ]),
  edgeCase('linear', 'arrow_cross', 'arrow_point', [
    [120, 80],
    [20, 10],
  ]),
  // Three points, one bend.
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [0, 40],
    [60, 40],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [60, 40],
    [0, 40],
    [0, 0],
  ]),
  edgeCase('linear', 'arrow_point', 'arrow_point', [
    [0, 0],
    [0, 40],
    [60, 40],
  ]),
  edgeCase('step', 'none', 'arrow_cross', [
    [0, 0],
    [40, 0],
    [40, 60],
  ]),
  edgeCase('monotoneX', 'arrow_circle', 'arrow_point', [
    [0, 30],
    [30, 30],
    [30, 0],
  ]),
  edgeCase('bumpX', 'arrow_cross', 'arrow_point', [
    [0, 0],
    [0, 50],
    [50, 50],
  ]),
  edgeCase('catmullRom', 'none', 'none', [
    [0, 0],
    [40, 0],
    [40, 40],
  ]),
  edgeCase('stepBefore', 'none', 'arrow_point', [
    [0, 0],
    [30, 0],
    [30, 30],
  ]),
  edgeCase('stepAfter', 'arrow_point', 'none', [
    [0, 0],
    [0, 30],
    [30, 30],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [0, 0],
    [200, 0],
  ]),
  edgeCase('rounded', 'none', 'none', [
    [0, 0],
    [0, 50],
  ]),
  // Four to seven points.
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [40, 0],
    [40, 40],
    [80, 40],
  ]),
  edgeCase('rounded', 'arrow_point', 'arrow_point', [
    [0, 60],
    [0, 20],
    [50, 20],
    [50, 80],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [90, 0],
    [90, 30],
    [20, 30],
    [20, 60],
  ]),
  edgeCase('cardinal', 'none', 'arrow_point', [
    [0, 0],
    [50, 0],
    [50, 50],
    [100, 50],
  ]),
  edgeCase('bumpX', 'arrow_point', 'none', [
    [0, 0],
    [30, 0],
    [30, 30],
    [60, 30],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [20, 0],
    [20, 30],
    [60, 30],
    [60, 60],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [0, 20],
    [30, 20],
    [30, 40],
    [60, 40],
    [60, 80],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [10, 0],
    [10, 20],
    [30, 20],
    [30, 0],
    [50, 0],
    [50, 40],
  ]),
  edgeCase('basis', 'none', 'arrow_point', [
    [60, 60],
    [60, 0],
    [0, 0],
  ]),
  // Collinear and repeated points.
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [20, 0],
    [50, 0],
    [50, 40],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [0, 0],
    [0, 0],
    [40, 0],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [40, 0],
    [40, 0],
    [40, 40],
  ]),
  // Segments of 0.1, 1, 4, 5, 6 and 8 px, alone and as legs of a bend.
  edgeCase('linear', 'none', 'arrow_point', [
    [0, 0],
    [0.1, 0],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [0, 1],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [0, 0],
    [4, 0],
  ]),
  edgeCase('rounded', 'none', 'arrow_cross', [
    [0, 0],
    [5, 0],
  ]),
  edgeCase('basis', 'none', 'arrow_circle', [
    [0, 0],
    [6, 0],
  ]),
  edgeCase('step', 'arrow_point', 'arrow_point', [
    [0, 0],
    [0, 8],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [0.1, 0],
    [0.1, 1],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [4, 0],
    [4, 4],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [5, 0],
    [5, 6],
  ]),
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [8, 0],
    [8, 1],
  ]),
  // Arrow pairs on short segments.
  edgeCase('linear', 'arrow_cross', 'arrow_cross', [
    [0, 0],
    [4, 0],
    [4, 30],
  ]),
  edgeCase('linear', 'arrow_circle', 'arrow_circle', [
    [0, 0],
    [5, 0],
  ]),
  edgeCase('rounded', 'none', 'arrow_cross', [
    [0, 0],
    [0, 8],
    [4, 8],
  ]),
  edgeCase('rounded', 'arrow_point', 'none', [
    [40, 0],
    [40, 40],
    [0, 40],
  ]),
  // Diagonal segments: one, and two.
  edgeCase('rounded', 'none', 'arrow_point', [
    [0, 0],
    [40, 10],
    [40, 50],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [0, 0],
    [30, 30],
    [60, 0],
  ]),
  edgeCase('rounded', 'arrow_cross', 'arrow_point', [
    [0, 0],
    [20, -20],
    [40, 0],
  ]),
  // Arbitrary decimals, and the tiny negative value that rounds to zero.
  edgeCase('rounded', 'none', 'arrow_point', [
    [10.12345, 20.5],
    [10.12345, 60.75],
    [70.5, 60.75],
  ]),
  edgeCase('linear', 'none', 'arrow_point', [
    [-0.0004, 0],
    [-0.0004, 30],
    [40, 30],
  ]),
];

/**
 * Crossing groups of line hops. Each group is run for both jump styles. The edges are named
 * a, b, c and d by their position in the group.
 */
const HOP_GROUPS: EdgeCase[][] = [
  // Orthogonal crossing.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 50],
      [100, 50],
    ]),
    edgeCase('linear', 'none', 'arrow_point', [
      [40, 0],
      [40, 100],
    ]),
  ],
  // One horizontal edge crosses two verticals.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 50],
      [120, 50],
    ]),
    edgeCase('linear', 'none', 'none', [
      [30, 0],
      [30, 100],
    ]),
    edgeCase('linear', 'none', 'arrow_point', [
      [90, 0],
      [90, 100],
    ]),
  ],
  // Crossing 4 px from a bend: the hop has no room and is dropped.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 40],
      [50, 40],
      [50, 100],
    ]),
    edgeCase('linear', 'none', 'none', [
      [46, 0],
      [46, 100],
    ]),
  ],
  // Crossing 12 px from a bend: the hop fits.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 40],
      [50, 40],
      [50, 100],
    ]),
    edgeCase('linear', 'none', 'none', [
      [38, 0],
      [38, 100],
    ]),
  ],
  // Crossing inside the corner of a rounded edge: the reference does not report it.
  [
    edgeCase('rounded', 'none', 'arrow_point', [
      [0, 40],
      [50, 40],
      [50, 100],
    ]),
    edgeCase('linear', 'none', 'none', [
      [48, 0],
      [48, 100],
    ]),
  ],
  // Two diagonals at 45 degrees: both are horizontal-dominant, so the later edge jumps.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 0],
      [100, 100],
    ]),
    edgeCase('linear', 'arrow_point', 'none', [
      [0, 100],
      [100, 0],
    ]),
  ],
  // Steep diagonal over a horizontal edge.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 50],
      [100, 50],
    ]),
    edgeCase('linear', 'none', 'none', [
      [20, 0],
      [80, 100],
    ]),
  ],
  // A rounded edge crossed by a linear edge.
  [
    edgeCase('rounded', 'none', 'arrow_point', [
      [0, 20],
      [100, 20],
      [100, 120],
    ]),
    edgeCase('linear', 'none', 'none', [
      [50, 0],
      [50, 60],
    ]),
  ],
  // Two rounded edges crossing twice, once in each orientation: both jump.
  [
    edgeCase('rounded', 'none', 'arrow_point', [
      [0, 20],
      [100, 20],
      [100, 120],
    ]),
    edgeCase('rounded', 'none', 'arrow_point', [
      [60, 0],
      [60, 80],
      [120, 80],
    ]),
  ],
  // Crowded: one horizontal edge crossed by three verticals, two of them 10 px apart.
  [
    edgeCase('linear', 'none', 'arrow_point', [
      [0, 50],
      [200, 50],
    ]),
    edgeCase('linear', 'none', 'none', [
      [30, 0],
      [30, 100],
    ]),
    edgeCase('linear', 'none', 'none', [
      [40, 0],
      [40, 100],
    ]),
    edgeCase('linear', 'none', 'arrow_point', [
      [150, 100],
      [150, 0],
    ]),
  ],
  // Rounded edge with arrows at both ends, crossed near its start.
  [
    edgeCase('rounded', 'arrow_cross', 'arrow_circle', [
      [0, 30],
      [90, 30],
      [90, 90],
    ]),
    edgeCase('linear', 'none', 'arrow_point', [
      [45, 0],
      [45, 60],
    ]),
  ],
  // Arrow cross at the start, which has no marker offset, crossed near the start.
  [
    edgeCase('linear', 'arrow_cross', 'arrow_point', [
      [0, 50],
      [60, 50],
    ]),
    edgeCase('linear', 'none', 'none', [
      [12, 0],
      [12, 100],
    ]),
  ],
  // A basis edge: the reference hops its polyline without the curve.
  [
    edgeCase('basis', 'none', 'arrow_point', [
      [0, 40],
      [100, 40],
    ]),
    edgeCase('linear', 'none', 'none', [
      [50, 0],
      [50, 80],
    ]),
  ],
];

/** Seeded PRNG (mulberry32). Returns numbers in [0, 1). */
function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const CURVES = [
  'rounded',
  'linear',
  'basis',
  'cardinal',
  'step',
  'stepBefore',
  'stepAfter',
  'natural',
  'monotoneX',
  'bumpX',
  'catmullRom',
] as const;
const START_ARROWS = ['none', 'arrow_point', 'arrow_cross', 'arrow_circle'] as const;
const END_ARROWS = [
  'arrow_point',
  'arrow_point',
  'arrow_point',
  'arrow_cross',
  'arrow_circle',
  'none',
] as const;
const GRID_STEPS = [0.5, 1] as const;
const SEGMENT_LENGTHS = [0.5, 1, 2, 4, 5, 6, 8, 10, 12, 20, 30] as const;

/**
 * Orthogonal routes from the seeded PRNG: two to seven points on a grid of 0.5 or 1 px. A segment
 * usually turns, sometimes continues the direction, and may go left or up.
 */
function generatedCases(count: number): EdgeCase[] {
  const random = mulberry32(GENERATOR_SEED);
  const choose = <T>(items: readonly T[]): T => items[Math.floor(random() * items.length)];
  const cases: EdgeCase[] = [];
  for (let k = 0; k < count; k++) {
    const grid = choose(GRID_STEPS);
    let x = Math.round(random() * 40) * grid;
    let y = Math.round(random() * 40) * grid;
    const points: Point[] = [{ x, y }];
    let vertical = random() < 0.5;
    const segments = 1 + Math.floor(random() * 6);
    for (let s = 0; s < segments; s++) {
      if (random() < 0.7) {
        vertical = !vertical;
      }
      const length = choose(SEGMENT_LENGTHS) * (random() < 0.5 ? -1 : 1);
      if (vertical) {
        y += length;
      } else {
        x += length;
      }
      points.push({ x, y });
    }
    cases.push({
      curve: choose(CURVES),
      arrowTypeStart: choose(START_ARROWS),
      arrowTypeEnd: choose(END_ARROWS),
      points,
    });
  }
  return cases;
}

/** Defines the globals that the reference reads, from a jsdom window. */
function installDom(): JSDOM {
  const dom = new JSDOM(`<svg xmlns="${SVG_NS}"></svg>`, { pretendToBeVisual: true });
  const { window } = dom;
  const define = (name: string, value: unknown): void => {
    Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  };
  define('window', window);
  define('document', window.document);
  const constructors = window as unknown as Record<string, unknown>;
  for (const name of [
    'Node',
    'Element',
    'HTMLElement',
    'SVGElement',
    'SVGGraphicsElement',
    'DOMParser',
    'XMLSerializer',
  ]) {
    if (!(name in globalThis)) {
      define(name, constructors[name]);
    }
  }
  return dom;
}

/** Calls the reference's `insertEdge` for one case and returns the `d` of the path it writes. */
function insertPath(d3: D3Module, edges: EdgesModule, document: Document, input: EdgeCase): string {
  const group = document.createElementNS(SVG_NS, 'g');
  document.documentElement.appendChild(group);
  edges.insertEdge(
    d3.select(group),
    {
      id: 'edge',
      points: input.points.map((p) => ({ ...p })),
      curve: input.curve,
      arrowTypeStart: input.arrowTypeStart,
      arrowTypeEnd: input.arrowTypeEnd,
      look: 'classic',
      thickness: 'normal',
      pattern: 'solid',
      classes: 'flowchart-link',
      style: [],
      cssCompiledStyles: [],
      arrowheadStyle: '',
    },
    new Map(),
    'flowchart-v2',
    {},
    {},
    'fixture',
    true
  );
  const path = group.querySelector('path');
  if (!path) {
    throw new Error('insertEdge wrote no path element');
  }
  const d = path.getAttribute('d');
  if (d === null) {
    throw new Error('insertEdge wrote a path without d');
  }
  return d;
}

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

function hopRecords(lineJump: LineJumpModule): LineHopRecord[] {
  const records: LineHopRecord[] = [];
  HOP_GROUPS.forEach((group, groupIndex) => {
    const id = `h${pad(groupIndex + 1)}`;
    const edges = group.map((edge, i) => ({
      id: HOP_EDGE_IDS[i],
      curve: edge.curve,
      arrowTypeStart: edge.arrowTypeStart,
      arrowTypeEnd: edge.arrowTypeEnd,
      points: edge.points.map((p) => ({ x: p.x, y: p.y })),
    }));
    const crossings = lineJump.findEdgeIntersections(edges);
    // Only the edges that jump are recorded, in the order the crossings report them.
    const jumping = [...new Set(crossings.map((c) => c.jumpEdgeId))];
    for (const jumpStyle of JUMP_STYLES) {
      const rewritten = lineJump.processEdgesWithJumps(edges, {
        enabled: true,
        jumpRadius: JUMP_RADIUS,
        jumpStyle,
      });
      const expected: Record<string, string> = {};
      for (const edgeId of jumping) {
        const d = rewritten.get(edgeId);
        if (d === undefined) {
          throw new Error(`${id}: processEdgesWithJumps returned no path for ${edgeId}`);
        }
        expected[edgeId] = d;
      }
      records.push({ id, jumpStyle, edges, expected });
    }
  });
  return records;
}

const dom = installDom();
const reference = await openReference();
try {
  const d3 = await reference.load<D3Module>(D3_MODULE);
  const edges = await reference.load<EdgesModule>(EDGES_MODULE);
  const lineJump = await reference.load<LineJumpModule>(LINE_JUMP_MODULE);

  const inputs = [...HAND_PICKED, ...generatedCases(GENERATED_PATHS)];
  const paths: PathRecord[] = [];
  inputs.forEach((input, index) => {
    try {
      const d = insertPath(d3, edges, dom.window.document, input);
      paths.push({
        id: `p${pad(paths.length + 1)}`,
        curve: input.curve,
        arrowTypeStart: input.arrowTypeStart,
        arrowTypeEnd: input.arrowTypeEnd,
        points: input.points,
        d,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`dropped path input ${index} (${input.curve}): ${message}`);
    }
  });

  const output = {
    generator: 'tools/port/export-flowchart-edges.ts',
    reference:
      'packages/mermaid/src/rendering-util/rendering-elements/edges.js insertEdge (jsdom), lineJump.ts processEdgesWithJumps',
    paths,
    lineHops: hopRecords(lineJump),
  };

  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(output, null, 2)}\n`);
  console.log(
    `wrote ${paths.length} paths and ${output.lineHops.length} line-hop entries to ${outputPath}`
  );
} finally {
  await reference.close();
  dom.window.close();
}
