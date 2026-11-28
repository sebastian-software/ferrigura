// Build a Go overlay that traces the exact graph around the pinned normalization stage.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [sourceRoot, output] = process.argv.slice(2);
assert(sourceRoot && output, 'usage: node trace-overlay.mjs <D2 checkout> <output directory>');
const pin = 'b463ea5d47272f4da6e1e08012dbb15a6a4dbcde';
const root = resolve(sourceRoot);
const dest = resolve(output);
const actual = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
assert.equal(actual, pin, 'D2 checkout must be at the pilot pin');

const source = join(root, 'd2layouts/d2talalayout/internal/engine/pipeline.go');
const target = join(dest, 'pipeline-overlay.go');
const imports = '\t"math/rand"\n';
const normalizer = '\tplacement.Normalize(p.graph)\n';
const original = await readFile(source, 'utf8');
assert.equal(original.split(imports).length, 2, 'Go imports have changed');
assert.equal(original.split(normalizer).length, 2, 'Go normalization call has changed');
const instrumented = original
  .replace(imports, imports + '\t"bytes"\n\t"os"\n\t"os/exec"\n')
  .replace(
    normalizer,
    '\ttracePilotNormalization("before", p.graph)\n' +
      '\tif err := runPilotNormalization(ctx, p.graph); err != nil { return err }\n' +
      '\ttracePilotNormalization("after", p.graph)\n'
  );
await mkdir(dest, { recursive: true });
const helper = await readFile(new URL('trace-normalization.go.txt', import.meta.url), 'utf8');
await writeFile(target, instrumented + '\n' + helper);
execFileSync('gofmt', ['-w', target]);
await writeFile(join(dest, 'overlay.json'), JSON.stringify({ Replace: { [source]: target } }));
