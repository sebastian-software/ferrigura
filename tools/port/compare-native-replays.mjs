import assert from 'node:assert/strict';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { JSDOM } from 'jsdom';
import { cases } from '../../node/scripts/replay-cases.mjs';

const directory = process.argv[2];
const destination = process.argv[3];
assert(directory && destination, 'Pass the artifact directory and report path');
const targets = JSON.parse(process.env.REPLAY_MATRIX)
  .include.map(({ id }) => id)
  .sort();
const expected = cases.map(({ name }) => name).sort();
const files = (await readdir(directory)).sort();
assert.deepEqual(
  files,
  targets.map((id) => `${id}.json`).sort(),
  'Missing or unexpected target artifacts'
);
const renders = new Map();
for (const target of targets) {
  const data = JSON.parse(await readFile(join(directory, `${target}.json`), 'utf8'));
  assert.deepEqual(Object.keys(data).sort(), expected, `${target}: missing or unexpected fixtures`);
  const fixtures = new Map();
  for (const name of expected) {
    assert.equal(typeof data[name], 'string', `${target}/${name}: missing SVG`);
    const document = new JSDOM(data[name], { contentType: 'image/svg+xml' }).window.document;
    assert.equal(document.documentElement.localName, 'svg', `${target}/${name}: invalid SVG`);
    assert.equal(document.documentElement.id, `${name}A`, `${target}/${name}: wrong SVG id`);
    fixtures.set(name, [...document.querySelectorAll('*')]);
  }
  renders.set(target, fixtures);
}
const base = targets.includes('linux-arm64-gnu') ? 'linux-arm64-gnu' : targets[0];
const geometry = [
  'x',
  'y',
  'x1',
  'x2',
  'y1',
  'y2',
  'width',
  'height',
  'cx',
  'cy',
  'r',
  'rx',
  'ry',
  'viewBox',
  'transform',
  'd',
  'points',
];
const numbers = /[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g;
const report = [
  `# Native replay comparison`,
  '',
  `Baseline: ${base}`,
  `Commit: ${process.env.GITHUB_SHA ?? 'local'}`,
  '',
  'Geometry is reported in SVG attribute units; no tolerance is applied.',
  '',
];
for (const target of targets.filter((id) => id !== base)) {
  report.push(`## ${target}`, '');
  for (const name of expected) {
    const left = renders.get(base).get(name);
    const right = renders.get(target).get(name);
    const structure = (elements) => {
      const index = new Map(elements.map((element, i) => [element, i]));
      return elements
        .map((element) => `${element.localName}:${index.get(element.parentElement) ?? -1}`)
        .join(',');
    };
    const tags = structure(left) === structure(right);
    const labels = (elements) =>
      elements
        .filter((element) => element.localName === 'text')
        .map((element) => element.textContent);
    const text = JSON.stringify(labels(left)) === JSON.stringify(labels(right));
    let max = 0;
    let where = 'none';
    let compared = 0;
    let incompatible = 0;
    if (tags) {
      for (let i = 0; i < left.length; i++) {
        for (const attribute of geometry) {
          const a = left[i].getAttribute(attribute);
          const b = right[i].getAttribute(attribute);
          if (a === null && b === null) continue;
          if (a === null || b === null || a.replace(numbers, '#') !== b.replace(numbers, '#')) {
            incompatible++;
            continue;
          }
          const av = a.match(numbers) ?? [];
          const bv = b.match(numbers) ?? [];
          if (av.length !== bv.length) {
            incompatible++;
            continue;
          }
          for (let n = 0; n < av.length; n++) {
            const delta = Math.abs(Number(av[n]) - Number(bv[n]));
            assert(Number.isFinite(delta), `${target}/${name}: non-finite geometry`);
            if (delta > max) {
              max = delta;
              where = `${left[i].localName}[${i}].${attribute}`;
            }
            compared++;
          }
        }
      }
    }
    report.push(
      `- ${name}: ordered elements ${tags ? 'match' : 'differ'} (${left.length}/${right.length}); text labels ${text ? 'match' : 'differ'}; numeric values compared ${compared}, max absolute delta ${max} at ${where}, incompatible geometry attributes ${incompatible}.`
    );
  }
  report.push('');
}
await writeFile(destination, `${report.join('\n')}\n`);
console.log(report.join('\n'));
