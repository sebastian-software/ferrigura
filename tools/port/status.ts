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
  timeline: ['timeline_spec', 'timeline_grammar'],
  info: ['info_spec', 'info_grammar'],
  journey: ['journey_spec', 'journey_db_spec'],
  er: ['er_spec', 'er_subgraph_spec', 'er_db_spec', 'er_characters_spec'],
  flowchart: [
    'flowchart_singlenode_spec',
    'flowchart_text_spec',
    'flowchart_md_string_spec',
    'flowchart_style_spec',
    'flowchart_edges_spec',
    'flowchart_arrows_spec',
    'flowchart_chaining_spec',
    'flowchart_lines_spec',
    'flowchart_comments_spec',
    'flowchart_direction_spec',
    'flowchart_subgraph_spec',
    'flowchart_flow_spec',
    'flowchart_interactions_spec',
    'flowchart_node_data_spec',
    'flowchart_db_spec',
    'flowchart_huge_spec',
  ],
  sequence: [
    'sequence_parse_spec',
    'sequence_statements_spec',
    'sequence_db_spec',
    'sequence_keyword_actors_spec',
    'sequence_render_spec',
  ],
  state: [
    'state_parser_spec',
    'state_db_spec',
    'state_style_spec',
    'state_diagram_v2_spec',
    'state_diagram_spec',
    'state_color_index_spec',
    'state_note_edge_spec',
    'state_note_node_spec',
    'state_renderer_spec',
  ],
};

interface Counts {
  ported: number;
  ignored: number;
  passing: number;
}

function countSuite(suite: string): Counts {
  // Cargo's own summary counts the tests that the macros generate, which a textual count misses.
  const run = spawnSync('cargo', ['test', '--quiet', '-p', 'ferrigura-syntax', '--test', suite], {
    cwd: root,
    encoding: 'utf8',
  });
  const summary = /test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/.exec(
    `${run.stdout}\n${run.stderr}`
  );
  const passing = summary ? Number(summary[1]) : 0;
  const failing = summary ? Number(summary[2]) : 0;
  const ignored = summary ? Number(summary[3]) : 0;
  return { ported: passing + failing + ignored, ignored, passing };
}

async function differentialCell(type: string): Promise<string> {
  try {
    const summary = JSON.parse(
      await readFile(`${root}target/port/${type}-summary.json`, 'utf8')
    ) as { inputs: number; differences: number; notYet?: number };
    const notYet = summary.notYet ? `, ${summary.notYet} not yet ported` : '';
    return summary.differences === 0
      ? `${summary.inputs} inputs, no difference${notYet}`
      : `${summary.differences} of ${summary.inputs} differ${notYet}`;
  } catch {
    return '—';
  }
}

const counts: Record<string, Counts> = {};
for (const [type, files] of Object.entries(suites)) {
  const total: Counts = { ported: 0, ignored: 0, passing: 0 };
  for (const file of files) {
    const count = countSuite(file);
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
