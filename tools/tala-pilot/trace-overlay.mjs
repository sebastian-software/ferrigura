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
};
assert(
  sourceRoot && output && Object.hasOwn(modes, mode),
  'usage: node trace-overlay.mjs <D2 checkout> <output directory> [normalization|ovg-seed|layout-graph|sizeless-placement]'
);
const pin = 'b463ea5d47272f4da6e1e08012dbb15a6a4dbcde';
const root = resolve(sourceRoot);
const dest = resolve(output);
const actual = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
assert.equal(actual, pin, 'D2 checkout must be at the pilot pin');

const [relativeSource, child, instrument] = modes[mode];
const source = join(root, 'd2layouts/d2talalayout', relativeSource);
const target = join(dest, child);
const original = await readFile(source, 'utf8');
const helper = await readFile(new URL(`trace-${mode}.go.txt`, import.meta.url), 'utf8');
const instrumented = instrument(original);
await mkdir(dest, { recursive: true });
await writeFile(target, instrumented + '\n' + helper);
execFileSync('gofmt', ['-w', target]);
await writeFile(join(dest, 'overlay.json'), JSON.stringify({ Replace: { [source]: target } }));

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

function traceOvgSeed(source) {
  const imports = '\t"math"\n';
  const intersections =
    '\tif err := ovg.addNodesIntersections(g, guard); err != nil {\n' +
    '\t\treturn nil, err\n\t}\n';
  return replaceOnce(
    replaceOnce(source, imports, imports + '\t"os"\n'),
    intersections,
    intersections + '\ttracePilotOVGSeed(ovg, g, len(hierarchyOVGs) == 0)\n'
  );
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
    imports + '\t"encoding/hex"\n\t"encoding/json"\n\t"os"\n'
  );
  const withFirstPass = replaceOnce(
    withImports,
    optimize,
    optimize + '\t\tif i == 0 { tracePilotSizelessPlacement("firstPass", seed, subgraph) }\n'
  );
  return replaceOnce(
    withFirstPass,
    transition,
    '\ttracePilotSizelessPlacement("fullPhase", seed, subgraph)\n' + transition
  );
}
