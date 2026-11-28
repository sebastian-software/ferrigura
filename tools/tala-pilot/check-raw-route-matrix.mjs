// Generate positioned fixtures with pinned Go, then compare each native path.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { copyFile, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const [source, binary] = process.argv.slice(2).map((path) => resolve(path));
assert(source && binary, 'usage: node check-raw-route-matrix.mjs <D2 checkout> <Rust>');

const scratch = await mkdtemp(join(tmpdir(), 'ferrigura-tala-route-matrix-'));
try {
  execFileSync(
    process.execPath,
    [fileURLToPath(new URL('./trace-raw-routes.mjs', import.meta.url)), source, scratch],
    { stdio: 'inherit' }
  );
  const overlayPath = join(scratch, 'overlay.json');
  const overlay = JSON.parse(await readFile(overlayPath, 'utf8'));
  const originalTest = join(source, 'd2layouts/d2talalayout/internal/routing/slingshot_test.go');
  const trialTest = join(scratch, 'raw_routing_matrix_test.go');
  await copyFile(new URL('./trace-raw-route-matrix.go.txt', import.meta.url), trialTest);
  overlay.Replace[originalTest] = trialTest;
  await writeFile(overlayPath, JSON.stringify(overlay));

  const output = execFileSync(
    'go',
    [
      'test',
      `-overlay=${overlayPath}`,
      './d2layouts/d2talalayout/internal/routing',
      '-run',
      '^TestPilotOrdinaryRawMatrix$',
      '-count=1',
      '-v',
    ],
    {
      cwd: source,
      encoding: 'utf8',
      env: {
        ...process.env,
        GOTOOLCHAIN: 'local',
        GOWORK: 'off',
        TALA_PILOT_TRACE_RAW_ROUTES: '1',
      },
      maxBuffer: 16_000_000,
      timeout: 120_000,
    }
  );

  const cases = [];
  const trialPrefix = 'TALA_PILOT_MATRIX_TRIAL ';
  const routePrefix = 'TALA_PILOT_RAW_ROUTES ';
  let current;
  for (const line of output.split('\n')) {
    if (line.startsWith(trialPrefix)) {
      current = { id: Number(line.slice(trialPrefix.length)), flavors: [] };
      cases.push(current);
    } else if (line.startsWith(routePrefix)) {
      assert(current, 'Go emitted a route outside a positioned fixture');
      const { input, ...flavor } = JSON.parse(line.slice(routePrefix.length));
      if (current.input) {
        assert.deepEqual(input, current.input, `Go inputs differed in fixture ${current.id}`);
      }
      current.input = input;
      current.flavors.push(flavor);
    }
  }
  assert.equal(cases.length, 128, 'Unexpected number of live positioned fixtures');
  let paths = 0;
  for (const fixture of cases) {
    assert.equal(fixture.flavors.length, 3, `Incomplete Go flavors for fixture ${fixture.id}`);
    const native = JSON.parse(
      execFileSync(binary, ['--raw-route-flavors'], {
        input: JSON.stringify(fixture.input),
        encoding: 'utf8',
        maxBuffer: 2_000_000,
        timeout: 10_000,
      })
    );
    assert.deepEqual(
      native.flavors,
      fixture.flavors,
      `Raw routing differs in fixture ${fixture.id}`
    );
    paths += fixture.flavors.reduce((total, flavor) => total + flavor.routes.length, 0);
  }
  assert.equal(paths, 1980, 'Unexpected number of live reference paths');
  console.log(`Matched ${paths} raw paths and costs in ${cases.length} positioned Go fixtures.`);
} finally {
  await rm(scratch, { recursive: true, force: true });
}
