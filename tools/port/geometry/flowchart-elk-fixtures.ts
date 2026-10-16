/**
 * Fixtures of the flowchart's ELK post-processing, from a capture of the reference.
 *
 * The capture (`pnpm port:geometry-flowchart --capture`) stores, for every flowchart fixture, the
 * graph the reference hands to elkjs (`elk-input.json`), the graph elkjs returns
 * (`elk-output.json`), and the reference's geometry (`reference-geometry.json`), each as a directory
 * `<capture>/<fixture>/`. This script selects fixtures that cover the post-processing passes and
 * writes what the Rust differential test reads, one JSON file per fixture, to the output directory
 * (`crates/ferrigura-render/tests/flowchart_elk_apply.rs` reads `crates/ferrigura-render/tests/data/flowchart-elk`):
 *
 * - `elk`: the elkjs output, reduced to the fields `apply` reads (`layoutOptions` and the rest are
 *   dropped);
 * - `sizes`: the nodes and edges of the input, with the measured sizes, the parents, the shapes and
 *   the title sizes that the post-processing reads;
 * - `reference`: the centres of the vertices, the frames of the subgraphs, the points of the edges
 *   (`dataPoints`) and the centres of the edge labels, as the reference painted them.
 *
 * Selection: a fixture is taken when it adds a coverage tag that fewer than `PER_TAG` selected
 * fixtures have. Fixtures whose leaves are rectangles come first, so that more edges are compared.
 * Excluded: hand-drawn fixtures (the reference paints their subgraph frames by rough.js, so their
 * frames are empty boxes), and fixtures whose painted edge labels move with a paint-only margin.
 *
 * Usage: `pnpm exec tsx tools/port/geometry/flowchart-elk-fixtures.ts --capture <dir> [--out <dir>]`
 */
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Fixtures taken per coverage tag, at most. */
const PER_TAG = 8;

/** Leaf shapes whose outline is the rectangle, as the reference's `intersect-rect` gives it. */
const RECTANGLE_SHAPES = new Set(['squareRect', 'roundedRect']);

/** Fixtures left out, with the reason. */
const EXCLUDED: Record<string, string> = {
  'visual-check-should-render-subgraphs-with-title-margins-and-edge-labels':
    'subGraphTitleMargin moves painted edge labels; it is paint, not layout',
};

interface ElkInputNode {
  id: string;
  isGroup?: boolean;
  shape?: string;
  padding?: number;
  width?: number;
  height?: number;
  labelData?: { width: number; height: number };
  look?: string;
  children?: ElkInputNode[];
}

interface ElkInputEdge {
  id: string;
  start: string;
  end: string;
  sources: string[];
  targets: string[];
  layoutReversed?: boolean;
  arrowTypeStart?: string;
  arrowTypeEnd?: string;
  label?: string;
  labels?: { width: number; height: number }[];
}

interface ElkInputRoot {
  layoutOptions?: Record<string, unknown>;
  children?: ElkInputNode[];
  edges?: ElkInputEdge[];
}

interface Point {
  x: number;
  y: number;
}

interface ElkOutputNode {
  id: string;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  children?: ElkOutputNode[];
}

interface ElkOutputEdge {
  id: string;
  sources?: string[];
  targets?: string[];
  sections?: {
    startPoint?: Point;
    endPoint?: Point;
    bendPoints?: Point[];
  }[];
  labels?: { x?: number; y?: number; width?: number; height?: number }[];
}

interface ElkOutputRoot extends ElkOutputNode {
  edges?: ElkOutputEdge[];
}

interface ReferenceGeometry {
  nodes: { id: string; centre: Point }[];
  clusters?: { id: string; rect: { x: number; y: number; width: number; height: number } }[];
  edges: { id: string; dataPoints: Point[] }[];
  edgeLabels?: { dataId: string; box: { x: number; y: number; width: number; height: number } }[];
}

/** One fixture as the Rust test reads it. */
interface FixtureFile {
  fixture: string;
  covers: string[];
  elk: ElkOutputRoot;
  sizes: {
    nodes: {
      id: string;
      parent: string | null;
      isGroup: boolean;
      shape: string;
      width: number | null;
      height: number | null;
      padding: number;
      labelWidth: number | null;
      labelHeight: number | null;
    }[];
    edges: {
      id: string;
      start: string;
      end: string;
      reversed: boolean;
      arrowTypeStart: string;
      arrowTypeEnd: string;
      label: string;
      width: number | null;
      height: number | null;
    }[];
  };
  reference: {
    nodes: Record<string, Point>;
    clusters: Record<string, { x: number; y: number; width: number; height: number }>;
    edges: Record<string, Point[]>;
    labels: Record<string, Point>;
  };
}

/** The fixture's data after the selection, before it is written. */
interface Candidate {
  slug: string;
  tags: string[];
  /** Whether every leaf is a rectangle, so that all of its edges can be compared. */
  rectangular: boolean;
  /** The edges whose two ends are rectangles or subgraphs. */
  rectangleEdges: number;
  nodeCount: number;
  file: FixtureFile;
}

/** The tags a fixture covers, from its input graph and its ELK output. */
function coverageTags(input: ElkInputRoot, output: ElkOutputRoot): string[] {
  const tags = new Set<string>();
  const direction = input.layoutOptions?.['elk.direction'];
  const directionTags: Record<string, string> = {
    RIGHT: 'direction:LR',
    LEFT: 'direction:RL',
    DOWN: 'direction:TB',
    UP: 'direction:BT',
  };
  tags.add(directionTags[String(direction)] ?? 'direction:other');

  const depth = new Map<string, number>();
  const isGroup = new Map<string, boolean>();
  const visit = (nodes: ElkInputNode[] | undefined, level: number): void => {
    for (const node of nodes ?? []) {
      depth.set(node.id, level);
      isGroup.set(node.id, node.isGroup === true);
      if (node.isGroup && (node.children ?? []).length === 0) {
        tags.add('empty-subgraph');
      }
      if (node.isGroup && level >= 1) {
        tags.add('nested-subgraph');
      }
      visit(node.children, level + 1);
    }
  };
  visit(input.children, 0);

  for (const edge of input.edges ?? []) {
    if (edge.layoutReversed) {
      tags.add('feedback');
    }
    if (edge.label) {
      tags.add('edge-label');
    }
    if (edge.start === edge.end) {
      tags.add('self-loop');
    }
    if (isGroup.get(edge.end)) {
      tags.add('edge-into-subgraph');
    }
    if (isGroup.get(edge.start)) {
      tags.add('edge-out-of-subgraph');
    }
    const levels = Math.abs((depth.get(edge.start) ?? 0) - (depth.get(edge.end) ?? 0));
    if (levels >= 2) {
      tags.add('edge-across-two-levels');
    }
  }

  for (const edge of output.edges ?? []) {
    for (const section of edge.sections ?? []) {
      if ((section.bendPoints ?? []).length > 0) {
        tags.add('bends');
      } else if (edge.sources?.[0] !== edge.targets?.[0]) {
        tags.add('straight');
      }
    }
  }
  return [...tags].sort();
}

/** The reference's id of a vertex: `ref-<slug>-flowchart-<id>-<n>`, the counter last. */
function vertexId(referenceId: string, slug: string): string | null {
  const body = referenceId.slice(`ref-${slug}-`.length);
  if (!body.startsWith('flowchart-')) {
    return null;
  }
  const withCounter = body.slice('flowchart-'.length);
  return withCounter.slice(0, withCounter.lastIndexOf('-'));
}

/** Whether any node of the input has the hand-drawn look. */
function usesHandDrawnLook(children: ElkInputNode[] | undefined): boolean {
  return (children ?? []).some(
    (node) => node.look === 'handDrawn' || usesHandDrawnLook(node.children)
  );
}

/** The output of elkjs, with the fields `apply` reads. */
function reduceOutput(node: ElkOutputNode): ElkOutputNode {
  const reduced: ElkOutputNode = { id: node.id };
  for (const key of ['x', 'y', 'width', 'height'] as const) {
    if (node[key] !== undefined) {
      reduced[key] = node[key];
    }
  }
  if (node.children) {
    reduced.children = node.children.map(reduceOutput);
  }
  return reduced;
}

function reduceEdges(edges: ElkOutputEdge[] | undefined): ElkOutputEdge[] {
  return (edges ?? []).map((edge) => ({
    id: edge.id,
    sources: edge.sources,
    targets: edge.targets,
    sections: (edge.sections ?? []).map((section) => ({
      startPoint: section.startPoint,
      endPoint: section.endPoint,
      bendPoints: section.bendPoints ?? [],
    })),
    labels: (edge.labels ?? []).map((label) => ({
      x: label.x,
      y: label.y,
      width: label.width,
      height: label.height,
    })),
  }));
}

/** The fixture file for one captured fixture. */
function buildFile(
  slug: string,
  input: ElkInputRoot,
  output: ElkOutputRoot,
  reference: ReferenceGeometry
): Candidate {
  const parents = new Map<string, string | null>();
  const nodes: FixtureFile['sizes']['nodes'] = [];
  const walk = (children: ElkInputNode[] | undefined, parent: string | null): void => {
    for (const node of children ?? []) {
      parents.set(node.id, parent);
      nodes.push({
        id: node.id,
        parent,
        isGroup: node.isGroup === true,
        shape: node.shape ?? '',
        width: node.isGroup ? null : (node.width ?? null),
        height: node.isGroup ? null : (node.height ?? null),
        padding: node.padding ?? 0,
        labelWidth: node.labelData?.width ?? null,
        labelHeight: node.labelData?.height ?? null,
      });
      walk(node.children, node.id);
    }
  };
  walk(input.children, null);

  const edges: FixtureFile['sizes']['edges'] = (input.edges ?? []).map((edge) => ({
    id: edge.id,
    start: edge.start,
    end: edge.end,
    reversed: edge.layoutReversed === true,
    arrowTypeStart: edge.arrowTypeStart ?? 'none',
    arrowTypeEnd: edge.arrowTypeEnd ?? 'none',
    label: edge.label ?? '',
    width: edge.labels?.[0]?.width ?? null,
    height: edge.labels?.[0]?.height ?? null,
  }));

  const referenceNodes: Record<string, Point> = {};
  for (const node of reference.nodes) {
    const id = vertexId(node.id, slug);
    if (id !== null) {
      referenceNodes[id] = { x: node.centre.x, y: node.centre.y };
    }
  }
  const clusters: FixtureFile['reference']['clusters'] = {};
  for (const cluster of reference.clusters ?? []) {
    clusters[cluster.id.slice(`ref-${slug}-`.length)] = { ...cluster.rect };
  }
  const referenceEdges: Record<string, Point[]> = {};
  for (const edge of reference.edges) {
    referenceEdges[edge.id.slice(`ref-${slug}-`.length)] = edge.dataPoints.map((p) => ({
      x: p.x,
      y: p.y,
    }));
  }
  const labels: Record<string, Point> = {};
  for (const label of reference.edgeLabels ?? []) {
    labels[label.dataId] = {
      x: label.box.x + label.box.width / 2,
      y: label.box.y + label.box.height / 2,
    };
  }

  const covers = coverageTags(input, output);
  const rectangle = (id: string | null): boolean => {
    const node = nodes.find((candidate) => candidate.id === id);
    return node !== undefined && (node.isGroup || RECTANGLE_SHAPES.has(node.shape));
  };
  const rectangleEdges = edges.filter(
    (edge) => rectangle(edge.start) && rectangle(edge.end)
  ).length;
  return {
    slug,
    tags: covers,
    rectangular: nodes.every((node) => node.isGroup || RECTANGLE_SHAPES.has(node.shape)),
    rectangleEdges,
    nodeCount: nodes.length,
    file: {
      fixture: slug,
      covers,
      elk: { ...reduceOutput(output), edges: reduceEdges(output.edges) },
      sizes: { nodes, edges },
      reference: { nodes: referenceNodes, clusters, edges: referenceEdges, labels },
    },
  };
}

/**
 * Picks the fixtures: rectangular ones first, then those with the most edges the test compares,
 * then the smaller ones. A fixture is taken when it adds a tag that fewer than `PER_TAG` selected
 * fixtures have.
 */
function select(candidates: Candidate[]): Candidate[] {
  const sorted = candidates
    .filter((candidate) => candidate.rectangleEdges > 0)
    .sort(
      (a, b) =>
        Number(!a.rectangular) - Number(!b.rectangular) ||
        b.rectangleEdges - a.rectangleEdges ||
        a.nodeCount - b.nodeCount ||
        a.slug.localeCompare(b.slug)
    );
  const counts = new Map<string, number>();
  const selected: Candidate[] = [];
  for (const candidate of sorted) {
    const needed = candidate.tags.some((tag) => (counts.get(tag) ?? 0) < PER_TAG);
    if (!needed) {
      continue;
    }
    selected.push(candidate);
    for (const tag of candidate.tags) {
      counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
  }
  return selected;
}

async function readJson<T>(path: string): Promise<T> {
  return JSON.parse(await readFile(path, 'utf8')) as T;
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const option = (name: string, fallback?: string): string | undefined => {
    const index = args.indexOf(name);
    return index === -1 ? fallback : args[index + 1];
  };
  const capture = option('--capture');
  if (!capture) {
    throw new Error(
      'Usage: tsx tools/port/geometry/flowchart-elk-fixtures.ts --capture <dir> [--out <dir>]'
    );
  }
  const repo = fileURLToPath(new URL('../../..', import.meta.url));
  const out = resolve(
    option('--out', join(repo, 'crates/ferrigura-render/tests/data/flowchart-elk'))!
  );

  const candidates: Candidate[] = [];
  let skippedHandDrawn = 0;
  for (const slug of (await readdir(capture)).sort()) {
    const dir = join(capture, slug);
    let input: ElkInputRoot[];
    try {
      input = await readJson<ElkInputRoot[]>(join(dir, 'elk-input.json'));
    } catch {
      continue;
    }
    if (EXCLUDED[slug]) {
      continue;
    }
    if (usesHandDrawnLook(input[0].children)) {
      skippedHandDrawn += 1;
      continue;
    }
    const output = (await readJson<ElkOutputRoot[]>(join(dir, 'elk-output.json')))[0];
    const reference = await readJson<ReferenceGeometry>(join(dir, 'reference-geometry.json'));
    candidates.push(buildFile(slug, input[0], output, reference));
  }

  const selected = select(candidates);
  await mkdir(out, { recursive: true });
  for (const candidate of selected) {
    await writeFile(join(out, `${candidate.slug}.json`), `${JSON.stringify(candidate.file)}\n`);
  }
  const coverage = new Map<string, number>();
  for (const candidate of selected) {
    for (const tag of candidate.tags) {
      coverage.set(tag, (coverage.get(tag) ?? 0) + 1);
    }
  }
  console.log(
    `${selected.length} fixtures of ${candidates.length} candidates (${skippedHandDrawn} hand-drawn left out) written to ${out}`
  );
  for (const [tag, count] of [...coverage].sort()) {
    console.log(`  ${tag}: ${count}`);
  }
}

await main();
