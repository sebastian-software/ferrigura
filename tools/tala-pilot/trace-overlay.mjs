// Build a Go overlay that traces a stage of the pinned reference.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [sourceRoot, output, mode = 'normalization'] = process.argv.slice(2);
assert(
  sourceRoot && output && ['normalization', 'ovg-seed'].includes(mode),
  'usage: node trace-overlay.mjs <D2 checkout> <output directory> [normalization|ovg-seed]'
);
const pin = 'b463ea5d47272f4da6e1e08012dbb15a6a4dbcde';
const root = resolve(sourceRoot);
const dest = resolve(output);
const actual = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
assert.equal(actual, pin, 'D2 checkout must be at the pilot pin');

const internal = join(root, 'd2layouts/d2talalayout/internal');
const source = join(internal, mode === 'ovg-seed' ? 'routing/ovg.go' : 'engine/pipeline.go');
const target = join(dest, mode === 'ovg-seed' ? 'ovg-overlay.go' : 'pipeline-overlay.go');
const original = await readFile(source, 'utf8');
const helper = await readFile(new URL(`trace-${mode}.go.txt`, import.meta.url), 'utf8');
const instrumented = mode === 'ovg-seed' ? traceOvgSeed(original) : traceNormalization(original);
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
