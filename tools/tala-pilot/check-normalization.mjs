// Replay exact Go stage inputs through the Rust port. Captures can be regenerated with a
// traced Go reference or checked offline from the committed fixture.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    rust: { type: 'string' },
    reference: { type: 'string' },
    measured: { type: 'string' },
    ferrigura: { type: 'string' },
    captures: { type: 'string' },
  },
});
assert(values.rust && values.captures, 'Provide --rust <binary> and --captures <JSON file>');
assert.equal(Boolean(values.reference), Boolean(values.measured), '--reference needs --measured');
assert(!values.ferrigura || values.reference, '--ferrigura needs --reference');
const referenceEnv = { ...process.env };
delete referenceEnv.TALA_PILOT_TRACE;
delete referenceEnv.TALA_PILOT_RUST_NORMALIZER;
const captureFile = resolve(values.captures);
const fixture = await readCaptures();
let checked = 0;
for (const { name, stages } of fixture) {
  assert(stages.length > 0 && stages.length % 2 === 0, `${name}: missing Go stage pair`);
  for (let index = 0; index < stages.length; index += 2) {
    const { stage: beforeStage, ...input } = stages[index];
    const { stage: afterStage, ...expected } = stages[index + 1];
    assert.equal(beforeStage, 'before');
    assert.equal(afterStage, 'after');
    assert(
      [...input.nodes, ...input.edges].every(({ id }) => Number.isSafeInteger(id)),
      `${name}: TALA entity ID cannot be round-tripped by the trace runner`
    );
    const normalized = runRust(input);
    assert.deepEqual(normalized, runRust(input), `${name}: Rust output changed across runs`);
    assert.deepEqual(
      comparedGeometry(normalized),
      comparedGeometry(expected),
      `${name}: layout geometry differs`
    );
    checked++;
  }
}
assert(checked > 0, 'No normalization stages checked');
if (values.reference) await writeFile(captureFile, `${JSON.stringify(fixture, null, 2)}\n`);
console.log(
  `Matched Rust and pinned Go node and route coordinates in ${checked} normalization stages.`
);
if (values.ferrigura) await verifyHybrid();

async function readCaptures() {
  if (!values.reference) return JSON.parse(await readFile(captureFile, 'utf8'));
  const prefix = 'TALA_PILOT_TRACE ';
  const result = [];
  for (const name of [
    'library-architecture',
    'publishing-flow',
    'observatory-services',
    'workshop-edges',
  ]) {
    const input = await readFile(join(values.measured, `${name}.measured.json`), 'utf8');
    const capture = spawnSync(resolve(values.reference), [], {
      input,
      env: { ...referenceEnv, TALA_PILOT_TRACE: '1' },
      encoding: 'utf8',
      maxBuffer: 8_000_000,
      timeout: 30_000,
    });
    assert.ifError(capture.error);
    assert.equal(capture.status, 0, capture.stderr);
    const stages = capture.stderr
      .split('\n')
      .filter((line) => line.startsWith(prefix))
      .map((line) => JSON.parse(line.slice(prefix.length)));
    result.push({ name, stages });
  }
  return result;
}

function runRust(input) {
  return JSON.parse(
    execFileSync(resolve(values.rust), [], {
      input: JSON.stringify(input),
      encoding: 'utf8',
      timeout: 30_000,
    })
  );
}

function comparedGeometry({ fixed, nodes, edges }) {
  // Go recomputes label anchors from normalized routes. The Rust port consumes the
  // original label anchors when choosing the origin and produces node and route positions.
  return { fixed, nodes, edges: edges.map(({ id, points }) => ({ id, points })) };
}

async function verifyHybrid() {
  const scratch = await mkdtemp(join(tmpdir(), 'tala-normalization-'));
  const resultPath = join(scratch, 'geometry.json');
  const ferrigura = resolve(values.ferrigura);
  try {
    for (const { name } of fixture) {
      const input = await readFile(join(values.measured, `${name}.measured.json`), 'utf8');
      const mermaid = new URL(`./fixtures/${name}.mmd`, import.meta.url);
      const run = (env) =>
        execFileSync(resolve(values.reference), [], { input, env, timeout: 30_000 });
      const baseline = run(referenceEnv);
      const hybrid = run({ ...referenceEnv, TALA_PILOT_RUST_NORMALIZER: resolve(values.rust) });
      assert.deepEqual(hybrid, baseline, `${name}: hybrid changed final geometry`);
      await writeFile(resultPath, baseline);
      const goSvg = execFileSync(ferrigura, ['paint', fileURLToPath(mermaid), resultPath], {
        timeout: 30_000,
      });
      await writeFile(resultPath, hybrid);
      const hybridSvg = execFileSync(ferrigura, ['paint', fileURLToPath(mermaid), resultPath], {
        timeout: 30_000,
      });
      assert.deepEqual(hybridSvg, goSvg, `${name}: hybrid changed final SVG`);
    }
  } finally {
    await rm(scratch, { recursive: true });
  }
  console.log(
    `Matched final Go and hybrid JSON and SVG byte for byte on ${fixture.length} flowcharts.`
  );
}
