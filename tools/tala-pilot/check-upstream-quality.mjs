// Compare the scored, completed graphs in the pinned D2 layout corpus.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    d2: { type: 'string' },
    overlay: { type: 'string' },
    rust: { type: 'string' },
    'write-fixtures': { type: 'boolean' },
  },
});
assert(values.d2 && values.overlay && values.rust, 'Pass --d2, --overlay and --rust.');
const d2 = resolve(values.d2);
const overlay = resolve(values.overlay);
const rust = resolve(values.rust);
const fixtures = new URL(
  '../../crates/ferrigura-tala/src/quality/go-completed-corpus.jsonl',
  import.meta.url
);
const bridge = await readFile(new URL('reference/build.sh', import.meta.url), 'utf8');
const pin = /^pin=([a-f0-9]{40})$/m.exec(bridge)?.[1];
assert(pin, 'The TALA Go bridge must declare its exact D2 pin.');
assert.equal(execFileSync('git', ['rev-parse', 'HEAD'], { cwd: d2, encoding: 'utf8' }).trim(), pin);
assert.equal(
  execFileSync('git', ['status', '--porcelain', '--untracked-files=all'], {
    cwd: d2,
    encoding: 'utf8',
  }).trim(),
  '',
  'Use the unchanged pinned checkout when running upstream fixture tests.'
);
const env = {
  ...process.env,
  GOFLAGS: '',
  GOTOOLCHAIN: 'local',
  GOWORK: 'off',
  TALA_TRACE_CANDIDATE_SCORE: '1',
};
const stdout = execFileSync(
  'go',
  [
    'test',
    '-mod=readonly',
    '-count=1',
    `-overlay=${overlay}`,
    '-run',
    '^(TestPilotCompletedQualityCorpus|TestPilotWorkScoringKernels)$',
    '-v',
    './d2layouts/d2talalayout',
  ],
  { cwd: d2, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }
);
const records = [];
let fixture;
for (const line of stdout.split('\n')) {
  assert(!line.startsWith('tala-candidate-error '), line);
  if (line.startsWith('tala-golden ')) {
    assert.equal(fixture, undefined, `${fixture}: missing candidate score`);
    fixture = line.slice('tala-golden '.length);
  }
  if (line.startsWith('tala-candidate ')) {
    assert(fixture, 'The Go trace must identify the completed graph.');
    const { graph, seed, penalty, area, work } = JSON.parse(line.slice('tala-candidate '.length));
    records.push({ fixture, graph, score: { penalty, area }, work });
    const actual = execFileSync(rust, ['--candidate-score'], {
      input: JSON.stringify([{ graph, seed }]),
      encoding: 'utf8',
    });
    assert.deepEqual(
      JSON.parse(actual).evaluations[0],
      { score: { penalty, area }, work },
      `${fixture}: completed score, area and work`
    );
    fixture = undefined;
  }
}
assert.equal(fixture, undefined, `${fixture}: missing candidate score`);
assert.equal(records.length, 35, 'Expected the 34 pinned goldens and representative work fixture.');
assert.equal(new Set(records.map((record) => record.fixture)).size, records.length);
const peak = records.reduce((best, record) => (record.work > best.work ? record : best));
if (values['write-fixtures']) {
  await writeFile(fixtures, `${records.map((record) => JSON.stringify(record)).join('\n')}\n`);
} else {
  const saved = (await readFile(fixtures, 'utf8'))
    .trimEnd()
    .split('\n')
    .map((line) => JSON.parse(line));
  assert.deepEqual(records, saved, 'Pinned completed-layout score fixtures drifted.');
}
console.log(
  `${records.length} upstream completed scores verified. Peak work: ${peak.work} (${peak.fixture}).`
);
