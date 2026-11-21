/**
 * Replay recorded swimlane layout inputs through the pinned reference's pure layout core.
 * This verifies the saved inputs and stages; it does not capture inputs from diagram renders or
 * regenerate the intermediate post-processing reports.
 *
 * pnpm exec tsx crates/swimlane/replay-corpus.ts [--corpus-dir <directory>]
 */
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { referenceRoot, repositoryRoot, requireReference } from '../../reference-root.js';

type Edge = { id: string; src: string; dst: string };
type Layout = {
  direction?: string;
  config: {
    flowchart?: { nodeSpacing?: number; rankSpacing?: number };
    swimlane?: {
      ignoreCrossLaneEdges?: boolean;
      optimizeRanksByCrossings?: boolean;
      automaticLaneOrdering?: boolean;
    };
  };
  edges: { points?: unknown }[];
};
type Result = {
  acyclic: { nodes: unknown; edges: Edge[] };
  reversed: Edge[];
  layering: { layers: unknown; rankOf: unknown; dummy: Iterable<unknown> };
  ordered: { layers: unknown };
  coordinates: unknown;
};
type Case = {
  name: string;
  input: Layout;
  stages: { stage: string; data: unknown }[];
  output: unknown;
  returnedDirection: string;
};

const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--corpus-dir')) {
  throw new Error('usage: replay-corpus.ts [--corpus-dir <directory>]');
}
const corpus = args.length
  ? resolve(args[1])
  : join(repositoryRoot, 'crates/ferrigura-swimlane/tests/corpus');
requireReference();
const source = join(
  referenceRoot,
  'packages/mermaid/src/rendering-util/layout-algorithms/swimlanes'
);
const { runSwimlaneLayoutCore } = (await import(
  pathToFileURL(join(source, 'layoutCore.ts')).href
)) as {
  runSwimlaneLayoutCore: (data: Layout) => string;
};
const { toGraphView, writeBackToLayoutData } = (await import(
  pathToFileURL(join(source, 'helpers.ts')).href
)) as {
  toGraphView: (data: Layout) => unknown;
  writeBackToLayoutData: (
    graph: unknown,
    ordered: unknown,
    coordinates: unknown,
    options: unknown
  ) => void;
};
const { sugiyamaLayout } = (await import(pathToFileURL(join(source, 'pipeline.ts')).href)) as {
  sugiyamaLayout: (graph: unknown, options: unknown) => Result;
};
const { routeEdgesOrthogonal } = (await import(
  pathToFileURL(join(source, 'orthogonalRouter/router.ts')).href
)) as {
  routeEdgesOrthogonal: (data: Layout, direction: string) => void;
};

/** The same Sugiyama fields that the Rust corpus test compares, without reference object aliases. */
function project(result: Result): unknown {
  const edges = (values: Edge[]) => values.map(({ id, src, dst }) => ({ id, src, dst }));
  return JSON.parse(
    JSON.stringify(
      {
        acyclic: { nodes: result.acyclic.nodes, edges: edges(result.acyclic.edges) },
        reversed: edges(result.reversed),
        layering: {
          layers: result.layering.layers,
          rankOf: result.layering.rankOf,
          dummy: [...result.layering.dummy],
        },
        ordered: { layers: result.ordered.layers },
        coordinates: result.coordinates,
      },
      (_key, value: unknown) => (value instanceof Map ? Object.fromEntries(value) : value)
    )
  );
}

const manifest = JSON.parse(await readFile(join(corpus, 'manifest.json'), 'utf8')) as {
  cases: { name: string }[];
};
const names = (await readdir(corpus))
  .filter((name) => name.endsWith('.json') && name !== 'manifest.json')
  .sort();
assert.ok(names.length > 0, 'the corpus must not be empty');
assert.deepEqual(
  names,
  manifest.cases.map(({ name }) => `${name}.json`).sort(),
  'corpus differs from manifest'
);
for (const file of names) {
  const item = JSON.parse(await readFile(join(corpus, file), 'utf8')) as Case;
  const stage = (name: string): unknown => {
    const found = item.stages.filter((entry) => entry.stage === name);
    assert.equal(found.length, 1, `${file}: expected one ${name} stage`);
    return found[0].data;
  };
  const output = structuredClone(item.input);
  const direction = runSwimlaneLayoutCore(output);
  assert.equal(direction, item.returnedDirection, `${file}: returned direction`);
  assert.deepEqual(output, item.output, `${file}: output`);

  const input = structuredClone(item.input);
  const graph = toGraphView(input);
  const nodeGap = input.config.flowchart?.nodeSpacing ?? 40;
  const layerGap = input.config.flowchart?.rankSpacing ?? 100;
  const layout = sugiyamaLayout(graph, {
    nodeGap,
    layerGap,
    ignoreCrossLaneEdges: input.config.swimlane?.ignoreCrossLaneEdges ?? true,
    optimizeRanksByCrossings: input.config.swimlane?.optimizeRanksByCrossings ?? true,
    automaticLaneOrdering: input.config.swimlane?.automaticLaneOrdering ?? false,
    direction: input.direction ?? 'TB',
  });
  assert.deepEqual(
    project(layout),
    project(stage('sugiyama') as Result),
    `${file}: sugiyama projection`
  );
  writeBackToLayoutData(graph, layout.ordered, layout.coordinates, { nodeGap, layerGap });
  assert.deepEqual(input, stage('writeback'), `${file}: writeback`);
  for (const edge of input.edges ?? []) delete edge.points;
  routeEdgesOrthogonal(input, input.direction ?? 'TB');
  assert.deepEqual(input, stage('routed'), `${file}: routed`);
}
console.log(`${names.length} recorded swimlane inputs matched the reference`);
