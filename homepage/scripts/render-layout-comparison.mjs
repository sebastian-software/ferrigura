/* eslint-disable security/detect-non-literal-fs-filename -- Case names come from the repository's comparison harness. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { format } from 'oxfmt';

const repository = new URL('../../', import.meta.url);
const binary = fileURLToPath(new URL('target/debug/examples/tala_pilot', repository));
const data = new URL('../app/data/layout-comparison.json', import.meta.url);
const assets = new URL('../public/layout-comparison/', import.meta.url);
const captures = fileURLToPath(new URL('tools/tala-pilot/captures/', repository));
const check = process.argv.includes('--check');
const nativeCases = new Set(['native-loop', 'native-boundary', 'native-parallel']);
const temporary = await mkdtemp(join(tmpdir(), 'ferrigura-layouts-'));

function run(command, args) {
  return execFileSync(command, args, {
    cwd: repository,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
  });
}

async function save(path, value) {
  if (check)
    assert.equal(await readFile(path, 'utf8'), value, `${path} changed. Run pnpm layouts:write.`);
  else await writeFile(path, value);
}

try {
  execFileSync(
    'cargo',
    [
      'build',
      '--locked',
      '-q',
      '-p',
      'ferrigura',
      '--features',
      'experimental-tala',
      '--example',
      'tala_pilot',
      '--target-dir',
      fileURLToPath(new URL('target/', repository)),
    ],
    { cwd: repository, stdio: 'inherit' }
  );
  run(process.execPath, [
    'tools/tala-pilot/compare.mjs',
    '--ferrigura',
    binary,
    '--captures',
    captures,
    '--out',
    temporary,
  ]);
  const samples = JSON.parse(await readFile(join(temporary, 'metrics.json'), 'utf8'));
  if (!check) await mkdir(assets, { recursive: true });
  const examples = [];
  for (const { id, title, results } of samples) {
    const measured = JSON.parse(await readFile(join(temporary, `${id}.measured.json`), 'utf8'));
    const sourcePath = join(temporary, `${id}.mmd`);
    const source = await readFile(sourcePath, 'utf8');
    const elk = await readFile(join(temporary, `${id}.elk.svg`), 'utf8');
    const reference = await readFile(join(temporary, `${id}.go.svg`), 'utf8');
    const native = nativeCases.has(id);
    const tala = native ? run(binary, ['tala', sourcePath]) : reference;
    if (native) assert.equal(tala, reference, `${id}: native and Go SVG outputs differ`);

    await save(new URL(`${id}.mmd`, assets), source);
    await save(new URL(`${id}-elk.svg`, assets), elk);
    await save(new URL(`${id}-tala.svg`, assets), tala);
    examples.push({
      id,
      title,
      source,
      native,
      nodes: measured.nodes.filter(({ isGroup }) => !isGroup).length,
      groups: measured.nodes.filter(({ isGroup }) => isGroup).length,
      edges: measured.edges.length,
      elk: results[0],
      tala: results[1],
    });
  }
  const pin = await readFile(new URL('tools/tala-pilot/reference/build.sh', repository), 'utf8');
  const reference = /^pin=([a-f0-9]{40})$/m.exec(pin)?.[1];
  assert(reference, 'The TALA bridge has no pinned revision');
  const output = await format('layout-comparison.json', JSON.stringify({ reference, examples }));
  assert.deepEqual(output.errors, []);
  await save(data, output.code);
  console.log(`Verified ${examples.length} ELK/TALA pairs against the checked-in layout captures.`);
} finally {
  await rm(temporary, { recursive: true, force: true });
}
