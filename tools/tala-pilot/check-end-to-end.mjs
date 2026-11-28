// Compare the native runtime with the unmodified Go engine on the same measured input.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual, parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    reference: { type: 'string' },
    ferrigura: { type: 'string' },
    rust: { type: 'string' },
    case: { type: 'string', multiple: true },
  },
});
assert(
  values.reference && values.ferrigura && values.rust,
  'Pass --reference, --ferrigura and --rust.'
);
const reference = resolve(values.reference);
const ferrigura = resolve(values.ferrigura);
const rust = resolve(values.rust);
const fixtures = fileURLToPath(new URL('fixtures/', import.meta.url));
const available = (await readdir(fixtures)).filter((name) => name.endsWith('.mmd')).sort();
const selected = values.case ?? available;
assert(selected.length > 0, 'No flowchart fixtures');
assert.equal(new Set(selected).size, selected.length, 'A fixture was selected more than once');
for (const name of selected) assert(available.includes(name), `Unknown fixture ${name}`);

const temp = await mkdtemp(join(tmpdir(), 'ferrigura-tala-parity-'));
const results = [];
try {
  for (const name of selected) {
    const source = join(fixtures, name);
    let measured;
    let capture;
    let expected;
    try {
      measured = run(ferrigura, ['measure', source]);
      capture = run(reference, ['--default'], measured);
      expected = geometry(JSON.parse(capture));
    } catch (error) {
      results.push({ name, status: 'oracle-input-error', detail: diagnostic(error) });
      continue;
    }
    let actual;
    try {
      actual = geometry(JSON.parse(run(rust, ['--layout'], measured)));
    } catch (error) {
      results.push({ name, status: 'rust-error', detail: diagnostic(error) });
      continue;
    }
    const difference = firstDifference(expected, actual);
    if (difference) {
      results.push({ name, status: 'geometry-mismatch', ...difference });
      continue;
    }
    const capturePath = join(temp, 'reference.json');
    await writeFile(capturePath, capture);
    let expectedSvg;
    try {
      expectedSvg = run(ferrigura, ['paint', source, capturePath]);
    } catch (error) {
      results.push({ name, status: 'oracle-paint-error', detail: diagnostic(error) });
      continue;
    }
    try {
      const actualSvg = run(ferrigura, ['tala', source]);
      results.push({ name, status: actualSvg === expectedSvg ? 'pass' : 'svg-mismatch' });
    } catch (error) {
      results.push({ name, status: 'rust-render-error', detail: diagnostic(error) });
    }
  }
} finally {
  await rm(temp, { recursive: true, force: true });
}

const passed = results.filter(({ status }) => status === 'pass').length;
console.log(
  JSON.stringify(
    { reference: 'D2 default layout', passed, total: results.length, results },
    null,
    2
  )
);
process.exitCode = passed === results.length ? 0 : 1;

function run(binary, args, input) {
  return execFileSync(binary, args, {
    encoding: 'utf8',
    input,
    stdio: ['pipe', 'pipe', 'pipe'],
    timeout: 60_000,
    maxBuffer: 8 * 1024 * 1024,
  });
}

function diagnostic(error) {
  return error.stderr?.trim() || error.message;
}

function geometry(layout) {
  return {
    nodes: layout.nodes,
    edges: layout.edges.map(({ id, points, label }) => ({ id, points, label: label ?? null })),
  };
}

function firstDifference(expected, actual) {
  for (const section of ['nodes', 'edges']) {
    const want = expected[section];
    const got = actual[section];
    if (want.length !== got.length) {
      return { path: `${section}.length`, expected: want.length, actual: got.length };
    }
    for (let index = 0; index < want.length; index++) {
      for (const field of Object.keys(want[index])) {
        if (!isDeepStrictEqual(want[index][field], got[index][field])) {
          return {
            path: `${section}[${index}].${field}`,
            id: want[index].id,
            expected: want[index][field],
            actual: got[index][field],
          };
        }
      }
    }
  }
}
