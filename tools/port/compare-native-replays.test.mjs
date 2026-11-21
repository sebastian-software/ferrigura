import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { cases } from '../../node/scripts/replay-cases.mjs';

const script = fileURLToPath(new URL('./compare-native-replays.mjs', import.meta.url));
const matrix = JSON.stringify({ include: [{ id: 'linux-arm64-gnu' }, { id: 'other' }] });
function fixture(x = 0, label = 'A') {
  return Object.fromEntries(
    cases.map(({ name }) => [
      name,
      `<svg xmlns="http://www.w3.org/2000/svg" id="${name}A" viewBox="${x} 0 10 10"><text>${label}</text></svg>`,
    ])
  );
}

test('reports structure, labels, and geometry and rejects incomplete captures', () => {
  const dir = mkdtempSync(join(tmpdir(), 'replay-report-'));
  const output = join(tmpdir(), `replay-report-${process.pid}.md`);
  const run = () =>
    spawnSync(process.execPath, [script, dir, output], {
      encoding: 'utf8',
      env: { ...process.env, REPLAY_MATRIX: matrix },
    });
  try {
    writeFileSync(join(dir, 'linux-arm64-gnu.json'), JSON.stringify(fixture()));
    writeFileSync(join(dir, 'other.json'), JSON.stringify(fixture(2, 'B')));
    assert.equal(run().status, 0);
    const report = readFileSync(output, 'utf8');
    assert.match(report, /ordered elements match/);
    assert.match(report, /text labels differ/);
    assert.match(report, /max absolute delta 2/);
    const missing = fixture();
    delete missing.venn;
    writeFileSync(join(dir, 'other.json'), JSON.stringify(missing));
    assert.notEqual(run().status, 0, 'A missing fixture must fail');
    rmSync(join(dir, 'other.json'));
    assert.notEqual(run().status, 0, 'A missing target must fail');
  } finally {
    rmSync(dir, { recursive: true, force: true });
    rmSync(output, { force: true });
  }
});
