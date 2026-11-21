import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

// Each invocation starts a fresh process and renderer. Compare only runs of the same binary on
// the same target; cross-target equality is a separate question.
const worker = fileURLToPath(new URL('../../scripts/render-replay.mjs', import.meta.url));

function renderInNewProcess(suffix) {
  const child = spawnSync(process.execPath, [worker, suffix], {
    encoding: 'utf8',
    timeout: 60_000,
    maxBuffer: 32 * 1024 * 1024,
  });
  assert.ifError(child.error);
  assert.equal(child.status, 0, child.stderr);
  return JSON.parse(child.stdout);
}

function assertReplay(first, second) {
  for (const [name, svg] of Object.entries(first)) {
    assert.equal(second[name], svg, `${name} differs between processes`);
  }
}

test('representative diagrams replay byte for byte in independent processes', () => {
  const first = renderInNewProcess('A');
  const second = renderInNewProcess('A');
  assert.deepEqual(Object.keys(first), [
    'flow-default',
    'flow-compat',
    'gantt',
    'venn',
    'architecture',
    'font',
  ]);
  assert.deepEqual(Object.keys(second), Object.keys(first));
  for (const [name, svg] of Object.entries(first)) {
    assert.match(svg, /<svg/);
    assert.match(svg, new RegExp(`id="${name}A"`));
  }
  assertReplay(first, second);
  assert.match(first.font, /data:font\//);
  const differentId = renderInNewProcess('B');
  assert.throws(() => assertReplay(first, differentId), /flow-default differs between processes/);
});
