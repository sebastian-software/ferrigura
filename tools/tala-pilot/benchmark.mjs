// Measure optimized Ferrigura and the pinned reference in isolated subprocesses.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readdir, readFile } from 'node:fs/promises';
import { availableParallelism, cpus, release, totalmem } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    ferrigura: { type: 'string', default: 'target/release/examples/tala_pilot' },
    reference: { type: 'string' },
    warmup: { type: 'string', default: '10' },
    samples: { type: 'string', default: '50' },
  },
});
const warmup = Number(values.warmup);
const samples = Number(values.samples);
assert(Number.isSafeInteger(warmup) && warmup >= 0, 'Use a nonnegative integer for --warmup.');
assert(Number.isSafeInteger(samples) && samples >= 1, 'Use a positive integer for --samples.');
const repo = new URL('../../', import.meta.url);
const ferrigura = resolve(values.ferrigura);
const reference = values.reference && resolve(values.reference);
const pin = await readFile(new URL('reference/build.sh', import.meta.url), 'utf8');
const revision = /^pin=([a-f0-9]{40})$/m.exec(pin)?.[1];
assert(revision, 'The TALA bridge has no pinned revision.');
const fixtureDirectory = new URL('fixtures/', import.meta.url);
const fixtures = (await readdir(fixtureDirectory)).filter((name) => name.endsWith('.mmd')).sort();

const report = {
  capturedAt: new Date().toISOString(),
  platform: process.platform,
  architecture: process.arch,
  system: release(),
  cpu: cpus()[0]?.model,
  logicalCpus: availableParallelism(),
  memoryBytes: totalmem(),
  warmup,
  samples,
  ferrigura: {
    baseline: readCommand('git', ['rev-parse', 'HEAD']),
    version: /^version = "([^"]+)"/m.exec(await readFile(new URL('Cargo.toml', repo), 'utf8'))?.[1],
    rustc: readCommand('rustc', ['-V']),
    profile: 'release',
    feature: 'experimental-tala',
  },
  goReference: reference
    ? {
        d2Version: '0.10.0',
        revision,
        toolchain: readCommand('go', ['version', reference]).split(/\s+/).at(-1),
        seed: 1,
        workers: 1,
      }
    : null,
  results: [],
};

for (const fixture of fixtures) {
  const source = fileURLToPath(new URL(fixture, fixtureDirectory));
  const measured = reference && execFileSync(ferrigura, ['measure', source], { encoding: 'utf8' });
  report.results.push({
    fixture,
    elkLayout: measure(ferrigura, ['benchmark', source, 'elk', 'layout', warmup, samples]),
    elkRender: measure(ferrigura, ['benchmark', source, 'elk', 'render', warmup, samples]),
    talaLayout: measure(ferrigura, ['benchmark', source, 'tala', 'layout', warmup, samples]),
    talaRender: measure(ferrigura, ['benchmark', source, 'tala', 'render', warmup, samples]),
    goLayout: reference ? measure(reference, [warmup, samples], measured) : null,
  });
}
console.log(JSON.stringify(report, null, 2));

function readCommand(command, args) {
  return execFileSync(command, args, { cwd: repo, encoding: 'utf8' }).trim();
}

function measure(command, args, input) {
  const timeOptions = { darwin: ['-l'], linux: ['-v'] }[process.platform];
  const options = { encoding: 'utf8', input, timeout: 60_000 };
  const raw = timeOptions
    ? spawnSync('/usr/bin/time', [...timeOptions, command, ...args.map(String)], options)
    : spawnSync(command, args.map(String), options);
  if (raw.error) throw raw.error;
  if (raw.status !== 0)
    return { error: raw.stderr.trim().split('\n').find(Boolean) ?? `exit status ${raw.status}` };
  const time = raw.stderr;
  const residentBytes =
    process.platform === 'darwin'
      ? Number(/^\s*(\d+)\s+maximum resident set size/m.exec(time)?.[1]) || null
      : (Number(/Maximum resident set size \(kbytes\):\s*(\d+)/.exec(time)?.[1]) || 0) * 1024 ||
        null;
  const times = JSON.parse(raw.stdout).sort((a, b) => a - b);
  assert.equal(times.length, samples);
  const middle = Math.floor(samples / 2);
  const medianNs = samples % 2 ? times[middle] : (times[middle - 1] + times[middle]) / 2;
  return { medianNs, p95Ns: times[Math.ceil(samples * 0.95) - 1], residentBytes };
}
