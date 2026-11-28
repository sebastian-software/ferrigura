// Add observational hooks to the pinned Go routing flavor and selection stages.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [checkout, output] = process.argv.slice(2);
assert(checkout && output, 'usage: node trace-raw-routes.mjs <D2 checkout> <output directory>');
const pin = 'b463ea5d47272f4da6e1e08012dbb15a6a4dbcde';
const root = resolve(checkout);
const destination = resolve(output);
const git = (args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
assert.equal(git(['rev-parse', 'HEAD']), pin, 'D2 checkout must be at the pilot pin');
assert.equal(
  git(['status', '--porcelain', '--untracked-files=all']),
  '',
  'D2 checkout must be clean'
);

const routing = join(root, 'd2layouts/d2talalayout/internal/routing');
const routerSource = join(routing, 'ovg_edge_router.go');
const coordinatorSource = join(routing, 'coordinator.go');
const pipelineSource = join(root, 'd2layouts/d2talalayout/internal/engine/pipeline.go');
const routerTarget = join(destination, 'pilot-raw-router.go');
const coordinatorTarget = join(destination, 'pilot-raw-coordinator.go');
const pipelineTarget = join(destination, 'pilot-raw-pipeline.go');

let router = await readFile(routerSource, 'utf8');
let coordinator = await readFile(coordinatorSource, 'utf8');
let pipeline = await readFile(pipelineSource, 'utf8');
const helper = await readFile(new URL('./trace-raw-routes.go.txt', import.meta.url), 'utf8');
router = replaceOnce(router, 'import (\n', 'import (\n\t"encoding/json"\n\t"os"\n\t"strconv"\n');
router = replaceOnce(
  router,
  '\ttotalDistance := 0.0\n\tfor _, edge := range router.edges {',
  '\tpilotTrace := startPilotRawRoutes(router)\n\ttotalDistance := 0.0\n\tfor _, edge := range router.edges {'
);
router = replaceOnce(
  router,
  '\t\tovgNodes, distance, err = router.slingshot(ctx, edge)',
  '\t\tpilotStrategy := "slingshot"\n\t\tovgNodes, distance, err = router.slingshot(ctx, edge)'
);
router = replaceOnce(
  router,
  '\t\t\tovgNodes, distance, err = router.search(ctx, edge)',
  '\t\t\tpilotStrategy = "search"\n\t\t\tovgNodes, distance, err = router.search(ctx, edge)'
);
const fallback = 'ovgNodes, distance, err = router.routeLine(ctx, edge)';
assert.equal(router.split(fallback).length, 3, 'Go route fallback anchors changed');
router = router.replaceAll(fallback, `pilotStrategy = "line"\n${fallback}`);
router = replaceOnce(
  router,
  '\t\ttotalDistance += distance',
  '\t\tpilotTrace.add(edge, pilotStrategy, distance, ovgNodes)\n\t\ttotalDistance += distance'
);
router = replaceOnce(
  router,
  '\tresp.Distance = totalDistance',
  '\tresp.Distance = totalDistance\n\tpilotTrace.finish(resp.Distance)'
);

coordinator = replaceOnce(
  coordinator,
  '\tapplyRouteUpdates(updates)\n\treturn nil',
  '\ttracePilotSelectedRoutes(g, work.flavor, updates)\n\tapplyRouteUpdates(updates)\n\treturn nil'
);

pipeline = replaceOnce(pipeline, 'import (\n', 'import (\n\t"os"\n');
pipeline = replaceOnce(
  pipeline,
  '\treturn routing.BalanceEdgeSegments(ctx, p.graph)\n',
  '\ttracePilotBalance("before", p.graph)\n' +
    '\terr := routing.BalanceEdgeSegments(ctx, p.graph)\n' +
    '\tif err == nil {tracePilotBalance("after", p.graph)}\n' +
    '\treturn err\n'
);
const balanceTrace = await readFile(
  new URL('./trace-route-balance.go.txt', import.meta.url),
  'utf8'
);

await mkdir(destination, { recursive: true });
await Promise.all([
  writeFile(routerTarget, `${router}\n${helper}`),
  writeFile(coordinatorTarget, coordinator),
  writeFile(pipelineTarget, `${pipeline}\n${balanceTrace}`),
]);
execFileSync('gofmt', ['-w', routerTarget, coordinatorTarget, pipelineTarget]);
await writeFile(
  join(destination, 'overlay.json'),
  JSON.stringify({
    Replace: {
      [routerSource]: routerTarget,
      [coordinatorSource]: coordinatorTarget,
      [pipelineSource]: pipelineTarget,
    },
  })
);

function replaceOnce(source, match, replacement) {
  assert.equal(source.split(match).length, 2, `Go trace anchor changed: ${match.trim()}`);
  return source.replace(match, replacement);
}
