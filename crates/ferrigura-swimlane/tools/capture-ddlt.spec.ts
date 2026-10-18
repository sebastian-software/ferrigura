/* eslint-disable no-console -- CLI-style spec: the part's summary goes to the console */
/**
 * The DDLT part of the swimlane corpus: every layout-test `.mmd` under
 * `e2e/platform/dev-diagrams/layout-tests/swimlanes/` that the DDLT harness can load, run through
 * `loadDdltFixture(<id>, { backendId: 'swimlanes' })` with the stage hook installed. The two
 * non-default option fixtures (`mermaid-work`, `14-messy-layout`) are excluded by the corpus format.
 *
 * It is run by `capture-corpus.ts`, or on its own (it writes `target/swimlane-corpus-parts/ddlt.json`,
 * or `$SWIMLANE_CORPUS_PARTS/ddlt.json`):
 *
 *   pnpm exec vitest run --config crates/ferrigura-swimlane/tools/vitest.config.ts
 */
import { mkdirSync, readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { describe, expect, it } from 'vitest';
import { loadDdltFixture } from '../../../packages/mermaid/src/rendering-util/layout-algorithms/ddlt/loadDdltFixture.js';
import { loadFreshSizesFixture } from '../../../packages/mermaid/src/rendering-util/layout-algorithms/ddlt/fixtureSizes.js';
import {
  casesFromCalls,
  identifierKeySet,
  PARTS_DIR,
  REPO_ROOT,
  splitCalls,
  type CaptureResult,
  type StageRecord,
} from './corpus-format.js';

const LAYOUT_TESTS_DIR = join(REPO_ROOT, 'e2e/platform/dev-diagrams/layout-tests/swimlanes');

type StageHook = (stage: string, data: unknown) => void;

function listFiles(dir: string, suffix: string): string[] {
  const found: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0
  )) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      found.push(...listFiles(full, suffix));
    } else if (entry.isFile() && entry.name.endsWith(suffix)) {
      found.push(full);
    }
  }
  return found;
}

/** Repository path, with `/` as the separator. */
function repoPath(file: string): string {
  return relative(REPO_ROOT, file).split(sep).join('/');
}

/** Case name: the path under the layout-tests `swimlanes/` folder, each segment turned into a slug, joined by `__`. */
function caseNameOf(file: string): string {
  return relative(LAYOUT_TESTS_DIR, file)
    .replace(/\.mmd$/, '')
    .split(sep)
    .map((segment) =>
      segment
        .toLowerCase()
        .replace(/[^\da-z]+/g, '-')
        .replace(/^-+|-+$/g, '')
    )
    .join('__');
}

/** The first line of the diagram after any front matter, must be `swimlane-beta`. */
function isSwimlaneSource(text: string): boolean {
  const body = text.replace(/^---[\S\s]*?\n---[^\n]*\n/, '');
  return /^\s*swimlane-beta\b/.test(body);
}

describe('swimlane corpus: DDLT part', () => {
  it('captures the DDLT layout inputs and outputs', async () => {
    const keys = new Set(identifierKeySet());
    const result: CaptureResult = { cases: [], excluded: [] };
    const sizesFiles = listFiles(LAYOUT_TESTS_DIR, '.sizes.json');
    const mmdFiles = listFiles(LAYOUT_TESTS_DIR, '.mmd');

    // A sizes file without its `.mmd`: the discovery reports it as missing and never loads it.
    for (const sizesPath of sizesFiles) {
      const mmdPath = sizesPath.replace(/\.sizes\.json$/, '.mmd');
      if (!existsSync(mmdPath)) {
        result.excluded.push({
          source: repoPath(sizesPath),
          reason:
            'sizes file without a .mmd: the DDLT discovery reports it as missing and skips it',
        });
      }
    }

    const hookGlobal = globalThis as typeof globalThis & { __mermaidSwimlaneStage?: StageHook };
    for (const mmdPath of mmdFiles) {
      const source = repoPath(mmdPath);
      const text = readFileSync(mmdPath, 'utf8');
      const sizesPath = mmdPath.replace(/\.mmd$/, '.sizes.json');
      const relId = `swimlanes/${relative(LAYOUT_TESTS_DIR, mmdPath)
        .replace(/\.mmd$/, '')
        .split(sep)
        .join('/')}`;
      if (!existsSync(sizesPath)) {
        result.excluded.push({
          source,
          reason:
            'no .sizes.json: the DDLT discovery (ddlt/discoverFixtures.ts) takes only fixtures with captured sizes, and the loader reads them',
        });
        continue;
      }
      try {
        loadFreshSizesFixture(sizesPath, mmdPath, relId);
      } catch (error) {
        result.excluded.push({
          source,
          reason: `sizes file fails the freshness check: ${String(error).slice(0, 300)}`,
        });
        continue;
      }
      if (!isSwimlaneSource(text)) {
        result.excluded.push({ source, reason: 'not a swimlane-beta diagram' });
        continue;
      }

      const records: StageRecord[] = [];
      hookGlobal.__mermaidSwimlaneStage = (stage, data) => {
        records.push({ stage, data });
      };
      let thrown: string | undefined;
      try {
        await loadDdltFixture(relId, { backendId: 'swimlanes' });
      } catch (error) {
        thrown = String(error).slice(0, 300);
      } finally {
        delete hookGlobal.__mermaidSwimlaneStage;
      }
      if (thrown !== undefined) {
        result.excluded.push({ source, reason: `the DDLT load threw: ${thrown}` });
        continue;
      }

      // A call that threw inside the core leaves no output stage, and its case is excluded with that reason.
      const part = casesFromCalls({
        baseName: caseNameOf(mmdPath),
        group: 'ddlt',
        source,
        text,
        calls: splitCalls(records),
        keys,
      });
      result.cases.push(...part.cases);
      result.excluded.push(...part.excluded);
    }

    const partsDir = process.env.SWIMLANE_CORPUS_PARTS ?? PARTS_DIR;
    mkdirSync(partsDir, { recursive: true });
    writeFileSync(join(partsDir, 'ddlt.json'), JSON.stringify(result));
    console.log(
      `DDLT part: ${result.cases.length} cases, ${result.excluded.length} exclusions (${partsDir})`
    );
    expect(result.cases.length).toBeGreaterThan(0);
  });
});
