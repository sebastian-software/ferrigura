/* eslint-disable security/detect-non-literal-fs-filename -- Paths are a fresh temporary directory. */
// What the scripts that write the site's drawings share: the published
// packages installed apart from the site's own dependency tree, and the
// comparison of a recorded file with a fresh render.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * Installs `dependencies` from npm into a temporary directory with lifecycle
 * scripts disabled and checks the installed versions. The caller removes the
 * directory with `remove` when it is done.
 */
export async function installPublished(dependencies, { quiet }) {
  const directory = await mkdtemp(join(tmpdir(), 'ferrigura-homepage-packages-'));
  const remove = () => rm(directory, { recursive: true, force: true });
  try {
    await writeFile(
      join(directory, 'package.json'),
      `${JSON.stringify({ private: true, type: 'module', dependencies }, null, 2)}\n`
    );
    execFileSync('npm', ['install', '--ignore-scripts', '--no-audit', '--no-fund'], {
      cwd: directory,
      stdio: quiet ? 'ignore' : 'inherit',
    });
    await checkVersions(directory, dependencies);
  } catch (error) {
    await remove();
    throw error;
  }
  return { directory, require: createRequire(join(directory, 'package.json')), remove };
}

async function checkVersions(directory, dependencies) {
  for (const [name, version] of Object.entries(dependencies)) {
    const manifest = join(directory, 'node_modules', name, 'package.json');
    const installed = JSON.parse(await readFile(manifest, 'utf8'));
    assert.equal(installed.version, version, `Installed ${name} ${installed.version}`);
  }
}

/**
 * Exact on the recording platform; elsewhere every number may move by 0.01,
 * as a few trigonometric results can differ in their last digits on another
 * architecture (docs/LEARNINGS.md, "fused multiply-adds").
 */
export function compareOutput({ actual, expected, label, exact, message }) {
  if (exact) {
    assert.equal(actual, expected, message);
    return;
  }
  const number = /-?\d[\d.]*/g;
  assert.equal(actual.replaceAll(number, '#'), expected.replaceAll(number, '#'), message);
  const left = actual.match(number) ?? [];
  const right = expected.match(number) ?? [];
  assert.equal(left.length, right.length, message);
  left.forEach((value, index) => {
    const other = right[index];
    // Version numbers and other dotted runs must match exactly.
    const near = Math.abs(Number(value) - Number(other)) <= 0.01;
    assert(value === other || near, `${message}: ${label} has ${value} for ${other}`);
  });
}
