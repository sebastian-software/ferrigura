// Build a Go overlay that traces a stage of the pinned reference.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [sourceRoot, output, mode = 'normalization'] = process.argv.slice(2);
const modes = {
  normalization: ['internal/engine/pipeline.go', 'pipeline-overlay.go', traceNormalization],
  'ovg-seed': ['internal/routing/ovg.go', 'ovg-overlay.go', traceOvgSeed],
  'sizeless-placement': [
    'internal/placement/node_placement.go',
    'sizeless-placement-overlay.go',
    traceSizelessPlacement,
  ],
  'layout-graph': ['seedgraph.go', 'seedgraph-overlay.go', traceLayoutGraph],
  'route-output': ['patch.go', 'patch-overlay.go', traceRouteOutput],
  'hierarchy-preprocess': [
    'internal/engine/pipeline.go',
    'hierarchy-preprocess-overlay.go',
    traceHierarchyPreprocess,
  ],
  'placement-bodies': [
    'internal/engine/pipeline.go',
    'placement-bodies-overlay.go',
    tracePlacementBodies,
  ],
  'edge-label-placement': [
    'internal/labeling/placement.go',
    'edge-label-placement-overlay.go',
    traceEdgeLabelPlacement,
  ],
};
assert(
  sourceRoot && output && (Object.hasOwn(modes, mode) || mode === 'candidate-score'),
  'usage: node trace-overlay.mjs <D2 checkout> <output directory> [normalization|ovg-seed|layout-graph|sizeless-placement|route-output|edge-label-placement|placement-bodies|hierarchy-preprocess|candidate-score]'
);
const pin = 'b463ea5d47272f4da6e1e08012dbb15a6a4dbcde';
const root = resolve(sourceRoot);
const dest = resolve(output);
const actual = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
assert.equal(actual, pin, 'D2 checkout must be at the pilot pin');

if (mode === 'candidate-score') {
  await traceCandidateScores(join(root, 'd2layouts/d2talalayout'));
  process.exit(0);
}

const [relativeSource, child, instrument] = modes[mode];
const source = join(root, 'd2layouts/d2talalayout', relativeSource);
const target = join(dest, child);
const original = await readFile(source, 'utf8');
const helper = await readFile(new URL(`trace-${mode}.go.txt`, import.meta.url), 'utf8');
const instrumented = instrument(original);
await mkdir(dest, { recursive: true });
await writeFile(target, instrumented + '\n' + helper);
execFileSync('gofmt', ['-w', target]);
const replace = { [source]: target };
if (mode === 'sizeless-placement') {
  const optimizer = join(root, 'd2layouts/d2talalayout/internal/placement/sized_optimizer.go');
  const tracedOptimizer = join(dest, 'sized-optimizer-overlay.go');
  await writeFile(tracedOptimizer, traceSizedOptimizer(await readFile(optimizer, 'utf8')));
  execFileSync('gofmt', ['-w', tracedOptimizer]);
  replace[optimizer] = tracedOptimizer;
}
await writeFile(join(dest, 'overlay.json'), JSON.stringify({ Replace: replace }));

function replaceOnce(source, match, replacement) {
  assert.equal(source.split(match).length, 2, `Go trace anchor changed: ${match.trim()}`);
  return source.replace(match, replacement);
}

function traceNormalization(source) {
  const imports = '\t"math/rand"\n';
  const normalizer = '\tplacement.Normalize(p.graph)\n';
  return replaceOnce(
    replaceOnce(source, imports, imports + '\t"bytes"\n\t"os"\n\t"os/exec"\n'),
    normalizer,
    '\ttracePilotNormalization("before", p.graph)\n' +
      '\tif err := runPilotNormalization(ctx, p.graph); err != nil { return err }\n' +
      '\ttracePilotNormalization("after", p.graph)\n'
  );
}

function tracePlacementBodies(source) {
  const imports = '\t"encoding/json"\n';
  const run = '\t\tif err := stage.run(p, ctx); err != nil {\n\t\t\treturn err\n\t\t}\n';
  return replaceOnce(
    replaceOnce(source, imports, imports + '\t"encoding/hex"\n\t"os"\n'),
    run,
    run + '\t\ttracePilotPlacementBodies(p, stage.name, stageIndex)\n'
  );
}

function traceOvgSeed(source) {
  const imports = '\t"math"\n';
  let instrumented = replaceOnce(source, imports, imports + '\t"os"\n');
  for (const [call, stage] of [
    ['addNodesIntersections(g, guard)', 'SEED'],
    ['addEdgesNodes(g, hierarchies, guard)', 'EDGE_POINTS'],
    ['addCornerNodes(g, tl, br, guard)', 'BOUNDARY_POINTS'],
    ['addTunnels(g, guard)', 'TUNNELS'],
    ['flagNodesNearPorts(guard)', 'LINKS'],
  ]) {
    const boundary = `\tif err := ovg.${call}; err != nil {\n\t\treturn nil, err\n\t}\n`;
    instrumented = replaceOnce(
      instrumented,
      boundary,
      boundary + `\ttracePilotOVGSeed(ovg, g, len(hierarchyOVGs) == 0, "${stage}")\n`
    );
  }
  return instrumented;
}

function traceLayoutGraph(source) {
  const imports = '\t"fmt"\n';
  const graphReady = '\tinternal.MarkSharedInput()\n';
  return replaceOnce(
    replaceOnce(
      source,
      imports,
      imports +
        '\t"encoding/json"\n\t"os"\n' +
        '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/placement"\n'
    ),
    graphReady,
    graphReady + '\ttracePilotLayoutGraph(ctx, graph, internal)\n'
  );
}

function traceRouteOutput(source) {
  const imports = '\t"fmt"\n';
  const append =
    '\t\tpatch.edges = append(patch.edges, edgePatch)\n\t\tpatch.finalEdges = append(patch.finalEdges, edge)\n';
  return replaceOnce(
    replaceOnce(source, imports, imports + '\t"encoding/json"\n\t"os"\n'),
    append,
    '\t\ttracePilotRouteOutput(edge, resultEdge)\n' + append
  );
}

function traceEdgeLabelPlacement(source) {
  const imports = '\t"fmt"\n';
  const boundary = '\treturn best, guard.check()\n';
  return replaceOnce(
    replaceOnce(source, imports, imports + '\t"encoding/json"\n\t"os"\n'),
    boundary,
    '\ttracePilotEdgeLabel(edge, g, ancestors, nonAncestors, placedFakeNodes, sharedSegmentFakeNodes, alternatives, best)\n' +
      boundary
  );
}

function traceSizelessPlacement(source) {
  const imports = '\t"math/rand"\n';
  const optimize =
    '\t\tif !idle {\n' +
    '\t\t\tif err := sizeless.optimize(ctx, temp); err != nil {\n' +
    '\t\t\t\treturn err\n\t\t\t}\n\t\t}\n';
  const transition =
    '\tif err := compaction(ctx, subgraph, compactionOptions{\n' +
    '\t\tedgeAbductions: edgeAbductions,\n\t\taxis:           horizontalAxis,';
  const withImports = replaceOnce(
    source,
    imports,
    imports +
      '\t"cmp"\n\t"encoding/hex"\n\t"encoding/json"\n\t"os"\n' +
      '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/limits"\n' +
      '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/placementcost"\n'
  );
  const withFirstPass = replaceOnce(
    withImports,
    optimize,
    optimize + '\t\tif i == 0 { tracePilotSizelessPlacement("firstPass", seed, subgraph, nil) }\n'
  );
  const withFullPhase = replaceOnce(
    withFirstPass,
    transition,
    '\ttracePilotSizelessPlacement("fullPhase", seed, subgraph, nil)\n' + transition
  );
  const aligned =
    '\tif err := validateGridAlignment(subgraph); err != nil {\n' + '\t\treturn err\n\t}\n';
  const withTransition = replaceOnce(
    withFullPhase,
    aligned,
    aligned + '\ttracePilotSizelessPlacement("sizedTransition", seed, subgraph, nil)\n'
  );
  const validatePlaced = '\t\tif err := validatePlacedNodes(root, subgraph.Nodes); err != nil {\n';
  const withDirection = replaceOnce(
    withTransition,
    validatePlaced,
    '\t\ttracePilotSizelessPlacement("directedComponent", []int64{randomSeed}, subgraph, nil)\n' +
      validatePlaced
  );
  const combined =
    '\tif err := orientSourceInterior(ctx, combined, root, ancestorObstacles); err != nil {\n\t\treturn err\n\t}\n';
  const withCombined = replaceOnce(
    withDirection,
    combined,
    combined +
      '\ttracePilotSizelessPlacement("placedComponent", []int64{randomSeed}, combined, nil)\n'
  );
  const withSeed = replaceOnce(
    withCombined,
    '\tdefer sized.release()\n',
    '\tsized.pilotSeed = seed\n\tdefer sized.release()\n'
  );
  const sizedCompaction =
    '\t\tif (i % compactionIteration) == 0 {\n' +
    '\t\t\t// These stages only move nodes and set herd fences:';
  const withFirstSizedPass = replaceOnce(
    withSeed,
    sizedCompaction,
    '\t\tif i == firstSized { tracePilotSizelessPlacement("firstSizedPass", seed, subgraph, nil) }\n' +
      '\t\ttracePilotSizedIteration("pass", i, seed, subgraph)\n' +
      sizedCompaction
  );
  const joinIslands =
    '\t\t\tif err := grouping.JoinDistancedClusters(ctx, subgraph); err != nil {\n' +
    '\t\t\t\treturn err\n\t\t\t}\n';
  const withIslandJoin = replaceOnce(
    withFirstSizedPass,
    joinIslands,
    '\t\t\tif os.Getenv("TALA_PILOT_TRACE_SIZED_ISLAND_JOIN") == "1" { tracePilotSizedIteration("beforeIslandJoin", i, seed, subgraph) }\n' +
      joinIslands
  );
  const afterCompaction =
    '\t\t\tproximity.SyncHerdFences(subgraph)\n' + '\t\t\tsized.endExternalChanges()\n';
  const withFirstCompaction = replaceOnce(
    withIslandJoin,
    afterCompaction,
    afterCompaction +
      '\t\t\ttracePilotSizedIteration("compaction", i, seed, subgraph)\n' +
      '\t\t\tif i - firstSized < compactionIteration { tracePilotSizelessPlacement("firstSizedCompaction", seed, subgraph, nil) }\n'
  );
  const settlingPasses =
    '\tfor i := 0; i < 10 && !idle; i++ {\n' +
    '\t\tchanged, err := sized.optimize(ctx, 0.0)\n' +
    '\t\tif err != nil {\n\t\t\treturn err\n\t\t}\n' +
    '\t\tif !changed {\n\t\t\tbreak\n\t\t}\n\t}\n';
  const tracedSettling = settlingPasses.replace(
    '\t\tif !changed {',
    '\t\ttracePilotSizedIteration("settlingPass", i, seed, subgraph)\n\t\tif !changed {'
  );
  return replaceOnce(
    withFirstCompaction,
    settlingPasses,
    tracedSettling + '\ttracePilotSizelessPlacement("sizedPhase", seed, subgraph, nil)\n'
  );
}

function traceSizedOptimizer(source) {
  const fields = 'type sizedOptimizer struct {\n';
  const shuffled =
    '\t\tif err := limits.Shuffle(points, optim.randGenerator, guard); err != nil {\n' +
    '\t\t\treturn false, err\n\t\t}\n';
  const tracedCandidates = replaceOnce(
    replaceOnce(source, fields, fields + '\tpilotSeed []int64\n'),
    shuffled,
    shuffled +
      '\t\tif optim.pilotSeed != nil {\n' +
      '\t\t\ttracePilotFirstSizedCandidates(ctx, optim, node, medianPoint, d, points)\n' +
      '\t\t}\n'
  );
  const moved = '\t\toptim.recordMove(node, moved)\n';
  return replaceOnce(
    tracedCandidates,
    moved,
    moved +
      '\t\tif optim.pilotSeed != nil && err == nil {\n' +
      '\t\t\ttracePilotSizelessPlacement("firstSizedMove", optim.pilotSeed, optim.g, map[string]float64{"x": node.TopLeft.X, "y": node.TopLeft.Y})\n' +
      '\t\t\toptim.pilotSeed = nil\n' +
      '\t\t}\n'
  );
}

function traceHierarchyPreprocess(source) {
  const imports = '\t"math/rand"\n';
  const run = '\t\tif err := stage.run(p, ctx); err != nil {\n\t\t\treturn err\n\t\t}\n';
  return replaceOnce(
    replaceOnce(source, imports, imports + '\t"os"\n\t"slices"\n'),
    run,
    run + '\t\ttracePilotHierarchyPreprocess(stage.name, p)\n'
  );
}

async function traceCandidateScores(tala) {
  const source = join(tala, 'layout.go');
  const target = join(dest, 'candidate-score-overlay.go');
  const qualitySource = join(tala, 'internal/quality/scoring.go');
  const qualityTarget = join(dest, 'quality-overlay.go');
  const testsSource = join(tala, 'result_test.go');
  const testsTarget = join(dest, 'quality-tests-overlay.go');
  const original = await readFile(source, 'utf8');
  const quality = await readFile(qualitySource, 'utf8');
  const helper = await readFile(new URL('trace-candidate-score.go.txt', import.meta.url), 'utf8');
  const tests = await readFile(testsSource, 'utf8');
  const testsHelper = await readFile(
    new URL('trace-candidate-quality-tests.go.txt', import.meta.url),
    'utf8'
  );
  const instrumentation = replaceOnce(
    replaceOnce(
      replaceOnce(
        original,
        '\t"errors"\n',
        '\t"errors"\n\t"encoding/json"\n\t"os"\n' +
          '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/labeling"\n' +
          '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/layoutgraph"\n' +
          '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/quality"\n' +
          '\t"github.com/d2lang/d2/lib/geo"\n' +
          '\t"github.com/d2lang/d2/lib/label"\n'
      ),
      '\tattempt.result, attempt.err = evaluateSeedResult(ctx, input, laidOut)\n',
      '\tattempt.result, attempt.err = evaluateSeedResult(ctx, input, laidOut)\n' +
        '\ttracePilotSeedCandidate(ctx, index, seed, attempt.result, attempt.err)\n'
    ),
    '\n\treturn best, nil\n}\n',
    '\n\ttracePilotSeedChoice(best.seed)\n\treturn best, nil\n}\n'
  );
  await mkdir(dest, { recursive: true });
  await writeFile(target, instrumentation + '\n' + helper);
  await writeFile(
    qualityTarget,
    quality +
      '\nfunc PilotEvaluationWork(ctx context.Context, graph *layoutgraph.Graph) (int64, error) {\n' +
      '\t_, _, work, err := evaluateWithAreaLimit(ctx, graph, maxEvaluationWorkUnits)\n' +
      '\treturn work, err\n}\n'
  );
  await writeFile(
    testsTarget,
    replaceOnce(
      tests,
      '\t"errors"\n',
      '\t"errors"\n\t"fmt"\n\t"os"\n\t"path/filepath"\n' +
        '\t"github.com/d2lang/d2/d2layouts/d2talalayout/internal/quality"\n'
    ) +
      '\n' +
      testsHelper
  );
  execFileSync('gofmt', ['-w', target, qualityTarget, testsTarget]);
  await writeFile(
    join(dest, 'overlay.json'),
    JSON.stringify({
      Replace: {
        [source]: target,
        [qualitySource]: qualityTarget,
        [testsSource]: testsTarget,
      },
    })
  );
}
