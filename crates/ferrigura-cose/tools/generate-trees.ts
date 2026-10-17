/**
 * Seeded generator of Mermaid mindmap texts for the cose-bilkent corpus (see capture-corpus.ts).
 *
 * Pure: it touches neither the browser nor the filesystem. The same seed and count always give the
 * same texts, so the corpus can be regenerated exactly. Each node is written two spaces deeper than
 * its parent, so the indentation carries the tree structure exactly.
 */

/** Seed of the generated set. Changing it changes every generated case. */
export const GENERATOR_SEED = 20261017;

/** Number of generated mindmaps. */
export const GENERATED_COUNT = 300;

/** Tree shapes. The cases cycle through them, so every shape type appears. */
export const TREE_SHAPES = ['random', 'path', 'star', 'broom', 'balanced'] as const;
export type TreeShape = (typeof TREE_SHAPES)[number];

/** Node shapes. None of them uses an icon, a class or markdown. */
export const NODE_SHAPES = [
  'default',
  'square',
  'rounded',
  'circle',
  'cloud',
  'bang',
  'hexagon',
] as const;
export type NodeShape = (typeof NODE_SHAPES)[number];

/**
 * How the node shapes of one tree are chosen: independently for each node, one shape for the whole
 * tree, or one shape per depth level.
 */
export const SHAPE_MODES = ['mixed', 'single', 'depth'] as const;
export type ShapeMode = (typeof SHAPE_MODES)[number];

/** A generated mindmap: the tree it was written from and the Mermaid text. */
export interface GeneratedMindmap {
  index: number;
  shape: TreeShape;
  shapeMode: ShapeMode;
  /** Parent of each node, in creation order. The root is node 0 and has parent -1. */
  parents: number[];
  labels: string[];
  nodeShapes: NodeShape[];
  text: string;
}

/**
 * Mulberry32, a small 32-bit PRNG. Returns floats in [0, 1). Fixed arithmetic, so the sequence
 * does not depend on the JavaScript engine.
 */
export function mulberry32(seed: number): () => number {
  let state = seed | 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function pick<T>(items: readonly T[], rng: () => number): T {
  return items[Math.floor(rng() * items.length)];
}

/**
 * Plain words only: lowercase letters and digits, no punctuation, and never the word "mindmap",
 * which the Mermaid lexer reads as the diagram keyword. Several are long enough to wrap.
 */
const WORDS = [
  'river',
  'signal',
  'garden',
  'compass',
  'harbour',
  'lantern',
  'meadow',
  'engine',
  'orbit',
  'ledger',
  'canvas',
  'pattern',
  'summit',
  'anchor',
  'beacon',
  'cipher',
  'delta',
  'ember',
  'forest',
  'glacier',
  'horizon',
  'island',
  'journey',
  'kernel',
  'ladder',
  'marble',
  'nectar',
  'oasis',
  'pixel',
  'quartz',
  'ripple',
  'saddle',
  'tunnel',
  'umbrella',
  'velvet',
  'willow',
  'yonder',
  'zephyr',
  'archive',
  'balance',
  'circuit',
  'doorway',
  'evidence',
  'firmware',
  'gradient',
  'hypothesis',
  'inventory',
  'junction',
  'keystone',
  'landscape',
  'migration',
  'notebook',
  'overview',
  'portfolio',
  'quotient',
  'roadmap',
  'standard',
  'telescope',
  'validation',
  'workflow',
  'interdisciplinary',
  'characterisation',
  'infrastructure',
  'multiplication',
  'responsibilities',
  'reproducibility',
  'scaffolding',
  'photographs',
  'v2',
  '3d',
  '2026',
];

/** Each Mermaid mindmap shape around a label. None of them is an icon, class or markdown. */
const SHAPE_WRAP: Record<NodeShape, (label: string) => string> = {
  default: (label) => label,
  square: (label) => `[${label}]`,
  rounded: (label) => `(${label})`,
  circle: (label) => `((${label}))`,
  cloud: (label) => `)${label}(`,
  bang: (label) => `))${label}((`,
  hexagon: (label) => `{{${label}}}`,
};

/** A label of 1 to 12 words; the longer ones wrap in the rendered node. */
function labelFor(rng: () => number): string {
  const wordCount = 1 + Math.floor(rng() * 12);
  const words = Array.from({ length: wordCount }, () => pick(WORDS, rng));
  if (rng() < 0.5) {
    words[0] = words[0].charAt(0).toUpperCase() + words[0].slice(1);
  }
  return words.join(' ');
}

/**
 * Node count of one case. Case 0 is a root alone, case 1 is the largest tree, a quarter of the
 * rest reach up to 150 nodes, and the others stay small (1 to 25) so that many cases are cheap.
 */
function nodeCountFor(index: number, rng: () => number): number {
  if (index === 0) {
    return 1;
  }
  if (index === 1) {
    return 150;
  }
  if (index % 4 === 3) {
    return 60 + Math.floor(rng() * 91);
  }
  return 1 + Math.floor(rng() * 25);
}

/**
 * Parent of each node, in creation order. Every parent has a smaller index than its child, which
 * the depth and traversal helpers rely on.
 */
export function parentsFor(shape: TreeShape, nodeCount: number, rng: () => number): number[] {
  const parents = [-1];
  // The broom is a handle path of half the nodes; the rest form a star on the handle's last node.
  const handle = Math.max(1, Math.ceil(nodeCount / 2));
  // Balanced trees are complete k-ary trees with k drawn as 2 or 3.
  const arity = shape === 'balanced' ? 2 + Math.floor(rng() * 2) : 2;
  for (let node = 1; node < nodeCount; node++) {
    switch (shape) {
      case 'random':
        parents.push(Math.floor(rng() * node));
        break;
      case 'path':
        parents.push(node - 1);
        break;
      case 'star':
        parents.push(0);
        break;
      case 'broom':
        parents.push(node < handle ? node - 1 : handle - 1);
        break;
      case 'balanced':
        parents.push(Math.floor((node - 1) / arity));
        break;
    }
  }
  return parents;
}

/** Depth of each node below the root. Throws when a parent does not come before its child. */
export function depthsOf(parents: readonly number[]): number[] {
  const depth = [0];
  for (let node = 1; node < parents.length; node++) {
    if (parents[node] < 0 || parents[node] >= node) {
      throw new Error(`node ${node} has parent ${parents[node]}, which does not come before it`);
    }
    depth.push(depth[parents[node]] + 1);
  }
  return depth;
}

/** Node indices in depth-first order, children in creation order. */
export function depthFirstOrder(parents: readonly number[]): number[] {
  const children: number[][] = parents.map(() => []);
  for (let node = 1; node < parents.length; node++) {
    children[parents[node]].push(node);
  }
  const order: number[] = [];
  const stack = [0];
  for (let next = stack.pop(); next !== undefined; next = stack.pop()) {
    order.push(next);
    for (let c = children[next].length - 1; c >= 0; c--) {
      stack.push(children[next][c]);
    }
  }
  return order;
}

function shapesFor(mode: ShapeMode, depth: readonly number[], rng: () => number): NodeShape[] {
  if (mode === 'single') {
    const shape = pick(NODE_SHAPES, rng);
    return depth.map(() => shape);
  }
  if (mode === 'depth') {
    return depth.map((level) => NODE_SHAPES[level % NODE_SHAPES.length]);
  }
  return depth.map(() => pick(NODE_SHAPES, rng));
}

/** Mermaid text: a `mindmap` line, then one line per node in depth-first order, indented by depth. */
function mindmapText(
  parents: readonly number[],
  labels: readonly string[],
  nodeShapes: readonly NodeShape[]
): string {
  const depth = depthsOf(parents);
  const lines = ['mindmap'];
  for (const node of depthFirstOrder(parents)) {
    const indent = ' '.repeat(2 * (depth[node] + 1));
    lines.push(`${indent}${SHAPE_WRAP[nodeShapes[node]](labels[node])}`);
  }
  return `${lines.join('\n')}\n`;
}

/** Throws unless the set covers every tree shape, shape mode and node shape, and a root-only case. */
export function assertCoverage(mindmaps: readonly GeneratedMindmap[]): void {
  const problems: string[] = [];
  for (const shape of TREE_SHAPES) {
    if (!mindmaps.some((m) => m.shape === shape)) {
      problems.push(`no ${shape} tree`);
    }
  }
  for (const mode of SHAPE_MODES) {
    if (!mindmaps.some((m) => m.shapeMode === mode)) {
      problems.push(`no ${mode} shape mode`);
    }
  }
  for (const shape of NODE_SHAPES) {
    if (!mindmaps.some((m) => m.nodeShapes.includes(shape))) {
      problems.push(`no ${shape} node`);
    }
  }
  if (!mindmaps.some((m) => m.parents.length === 1)) {
    problems.push('no root-only mindmap');
  }
  if (problems.length > 0) {
    throw new Error(`generator coverage: ${problems.join('; ')}`);
  }
}

/** The generated set: `count` mindmaps from one seeded stream, checked for coverage. */
export function generateMindmaps(
  seed: number = GENERATOR_SEED,
  count: number = GENERATED_COUNT
): GeneratedMindmap[] {
  const rng = mulberry32(seed);
  const mindmaps: GeneratedMindmap[] = [];
  for (let index = 0; index < count; index++) {
    const shape = TREE_SHAPES[index % TREE_SHAPES.length];
    const nodeCount = nodeCountFor(index, rng);
    const parents = parentsFor(shape, nodeCount, rng);
    const shapeMode = pick(SHAPE_MODES, rng);
    const labels = Array.from({ length: nodeCount }, () => labelFor(rng));
    const nodeShapes = shapesFor(shapeMode, depthsOf(parents), rng);
    mindmaps.push({
      index,
      shape,
      shapeMode,
      parents,
      labels,
      nodeShapes,
      text: mindmapText(parents, labels, nodeShapes),
    });
  }
  assertCoverage(mindmaps);
  return mindmaps;
}
