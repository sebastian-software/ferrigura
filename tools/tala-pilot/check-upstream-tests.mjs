import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    d2: { type: 'string' },
    json: { type: 'boolean' },
    verify: { type: 'boolean' },
    'require-complete': { type: 'boolean' },
  },
});
assert(values.d2, 'Pass --d2 with the path to the pinned D2 checkout.');

const root = fileURLToPath(new URL('../../', import.meta.url));
const d2 = resolve(values.d2);
const scope = 'd2layouts/d2talalayout';
const pin = /^pin=([a-f0-9]{40})$/m.exec(readLocal('reference/build.sh'))?.[1];
assert(pin, 'The TALA Go bridge must declare its exact D2 pin.');
assert.equal(run('git', ['rev-parse', 'HEAD'], d2).trim(), pin, 'Wrong D2 checkout revision.');

const names = '(Test|Fuzz|Benchmark)[A-Z0-9_][A-Za-z0-9_]*|Example[A-Za-z0-9_]*';
const declaration = new RegExp(`^func (${names})\\s*\\(`);
const matches = run(
  'git',
  ['grep', '-z', '-n', '-E', `^func (${names})[[:space:]]*[(]`, pin, '--', `${scope}/*_test.go`],
  d2
);
const mappings = new Map(Object.entries(JSON.parse(readLocal('upstream-test-map.json'))));
const tests = matches
  .trimEnd()
  .split('\n')
  .map((match) => sourceTest(match, mappings, declaration));
assert.equal(mappings.size, 0, `Unknown Go tests in map: ${[...mappings.keys()].join(', ')}`);

const pending = tests.filter(({ status }) => status === 'pending').length;
const verify = values.verify || values['require-complete'];
let mappedTestsVerified = false;
let verificationError;
if (verify) {
  try {
    verifyMappedTests(tests);
    mappedTestsVerified = true;
  } catch (error) {
    verificationError = error;
  }
}

if (values.json) {
  const complete = pending === 0 && mappedTestsVerified;
  console.log(
    JSON.stringify(
      {
        revision: pin,
        scope,
        mappedTestsVerified,
        complete,
        pending,
        verificationError: verificationError?.message,
        tests,
      },
      null,
      2
    )
  );
} else {
  const packages = new Map();
  for (const { package: directory, status } of tests) {
    const counts = packages.get(directory) ?? { mapped: 0, pending: 0, benchmark: 0 };
    counts[status]++;
    packages.set(directory, counts);
  }
  console.log(`D2 TALA test declarations at ${pin}`);
  console.table(Object.fromEntries(packages));
  console.log(
    `${tests.filter(({ status }) => status === 'mapped').length} mapped, ${pending} pending.`
  );
}
if (verificationError) {
  console.error(verificationError.message);
  process.exitCode = 1;
}
if (values['require-complete'] && pending > 0) {
  console.error(`${pending} upstream TALA correctness declarations still have no Rust mapping.`);
  process.exitCode = 1;
}

function readLocal(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

function sourceTest(match, map, pattern) {
  const [qualifiedPath, line, source] = match.split('\0');
  const file = qualifiedPath.slice(pin.length + scope.length + 2);
  const name = pattern.exec(source)?.[1];
  assert(name, `Missing test name in ${file}:${line}`);
  const id = `${file}#${name}`;
  const rust = map.get(id);
  map.delete(id);
  const benchmark = name.startsWith('Benchmark');
  if (rust !== undefined) {
    assert(!benchmark, `Benchmarks are measured separately: ${id}`);
    assert(Array.isArray(rust) && rust.length > 0, `No Rust tests mapped for ${id}`);
    assert(
      rust.every((test) => typeof test === 'string' && /^\w+(?:::\w+)*$/.test(test)),
      id
    );
  }
  return {
    package: dirname(file).replaceAll('\\', '/'),
    file,
    line: Number(line),
    name,
    status: rust ? 'mapped' : benchmark ? 'benchmark' : 'pending',
    rust: rust ?? [],
  };
}

function verifyMappedTests(tests) {
  assert.equal(
    run('git', ['status', '--porcelain', '--untracked-files=all'], d2).trim(),
    '',
    'Use a clean D2 tree when executing upstream tests.'
  );
  const goEnv = { ...process.env, GOFLAGS: '', GOWORK: 'off', GOTOOLCHAIN: 'local' };
  const goMod = run('git', ['show', `${pin}:go.mod`], d2);
  const toolchain = /^go (\S+)$/m.exec(goMod)?.[1];
  assert(toolchain, 'D2 must declare its Go version.');
  assert.equal(
    run('go', ['env', 'GOVERSION'], d2, goEnv).trim(),
    `go${toolchain}`,
    'Use the Go version pinned by D2.'
  );
  const packages = new Map();
  const rust = new Set();
  for (const test of tests.filter(({ status }) => status === 'mapped')) {
    const names = packages.get(test.package) ?? [];
    names.push(test.name);
    packages.set(test.package, names);
    for (const name of test.rust) rust.add(name);
  }
  assert(rust.size > 0, 'There are no mapped Rust tests to verify.');
  for (const [directory, names] of packages) {
    const path = directory === '.' ? `./${scope}` : `./${scope}/${directory}`;
    const output = run(
      'go',
      ['test', '-mod=readonly', '-count=1', '-json', '-run', `^(${names.join('|')})$`, path],
      d2,
      goEnv
    );
    const results = output
      .trim()
      .split('\n')
      .map((line) => JSON.parse(line));
    const skipped = results
      .filter((event) => event.Action === 'skip' && event.Test)
      .map((event) => event.Test);
    assert.equal(
      skipped.length,
      0,
      `Mapped Go cases were skipped in ${path}: ${skipped.join(', ')}`
    );
    for (const name of names) {
      assert(
        results.some((event) => event.Action === 'pass' && event.Test === name),
        `Mapped Go test did not run successfully: ${path} ${name}`
      );
    }
  }
  const output = run('cargo', ['test', '--locked', '-p', 'ferrigura-tala', '--lib'], root, {
    ...process.env,
    CARGO_TERM_COLOR: 'never',
  });
  const lines = new Set(output.split(/\r?\n/));
  for (const name of rust) {
    assert(lines.has(`test ${name} ... ok`), `Mapped Rust test did not run successfully: ${name}`);
  }
}

function run(command, args, cwd, env) {
  return execFileSync(command, args, { cwd, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
}
