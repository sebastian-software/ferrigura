/**
 * Counts the ported Rust specs per diagram type and rewrites the P1 table of `docs-port/STATUS.md`:
 * the ported, ignored and passing tests of each suite, and the result of the last differential
 * run (`pnpm port:diff`) read from `target/port/`.
 *
 * Usage: pnpm port:status
 */
import { spawnSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const statusPath = `${root}docs-port/STATUS.md`;

/** The ported suites of each diagram type, as file names in `crates/ferrigura-syntax/tests/`. */
const suites: Record<string, string[]> = {
  pie: ['pie_spec', 'pie_grammar'],
  info: ['info_spec', 'info_grammar'],
};

interface Counts {
  ported: number;
  ignored: number;
  passing: number;
}

async function countSuite(suite: string): Promise<Counts> {
  const source = await readFile(`${root}crates/ferrigura-syntax/tests/${suite}.rs`, 'utf8');
  const ported = source.match(/#\[test]/g)?.length ?? 0;
  const ignored = source.match(/#\[ignore/g)?.length ?? 0;

  const run = spawnSync('cargo', ['test', '--quiet', '-p', 'ferrigura-syntax', '--test', suite], {
    cwd: root,
    encoding: 'utf8',
  });
  const summary = /test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/.exec(
    `${run.stdout}\n${run.stderr}`
  );
  const passing = summary ? Number(summary[1]) : 0;
  return { ported, ignored, passing };
}

async function differentialCell(type: string): Promise<string> {
  try {
    const summary = JSON.parse(
      await readFile(`${root}target/port/${type}-summary.json`, 'utf8')
    ) as { inputs: number; differences: number };
    return summary.differences === 0
      ? `${summary.inputs} inputs, no difference`
      : `${summary.differences} of ${summary.inputs} differ`;
  } catch {
    return '—';
  }
}

const counts: Record<string, Counts> = {};
for (const [type, files] of Object.entries(suites)) {
  const total: Counts = { ported: 0, ignored: 0, passing: 0 };
  for (const file of files) {
    const count = await countSuite(file);
    total.ported += count.ported;
    total.ignored += count.ignored;
    total.passing += count.passing;
  }
  counts[type] = total;
}

const status = await readFile(statusPath, 'utf8');
const differential: Record<string, string> = {};
for (const type of Object.keys(suites)) {
  differential[type] = await differentialCell(type);
}

/** Rewrites the parser, specs and differential cells of one P1 row. */
function rewriteRow(line: string, type: string): string {
  const cells = line.split('|');
  const { ported, ignored, passing } = counts[type];
  // Cells: 0 is empty, 1 the type, then Grammar, Spec files, ≈ Tests, Fixtures, Parser,
  // Specs ported / passing, Differential parse, Layout, Render, Parity, and empty.
  cells[6] = ' ferrigura-syntax ';
  cells[7] = ` ${passing} / ${ported}${ignored > 0 ? ` (${ignored} ignored)` : ''} `;
  cells[8] = ` ${differential[type]} `;
  return cells.join('|');
}

const rewritten = status
  .split('\n')
  .map((line) => {
    for (const type of Object.keys(suites)) {
      if (line.startsWith(`| ${type} `)) {
        return rewriteRow(line, type);
      }
    }
    return line;
  })
  .join('\n');

await writeFile(statusPath, rewritten);
// The table's column widths follow the formatter, so the file is formatted after the rewrite.
spawnSync('pnpm', ['exec', 'prettier', '--write', statusPath], { cwd: root, stdio: 'ignore' });
for (const [type, count] of Object.entries(counts)) {
  console.log(
    `${type}: ${count.passing} passing of ${count.ported} ported, ${count.ignored} ignored; ` +
      `differential: ${differential[type]}`
  );
}
