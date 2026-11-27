/**
 * Counts the ported Rust specs per diagram type and rewrites the coverage table of
 * `docs/diagrams/README.md`: the ported, ignored and passing tests of each suite, and the results of
 * the last differential runs (`pnpm -C tools/port port:diff`, read from `target/port/`) and of the
 * last geometry run (`pnpm -C tools/port port:geometry`, read from `target/geometry/`). A cell whose
 * summary this checkout does not have keeps what the table says, so the script is safe to run where
 * the reference is not built.
 *
 * Usage: pnpm -C tools/port port:status
 */
import { spawnSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const tablePath = `${root}docs/diagrams/README.md`;

/**
 * The ported suites of each diagram type, as file names in `crates/ferrigura-syntax/tests/`, or in
 * `crates/ferrigura-render/tests/` with the prefix `render:` (the specs of the layout data and the
 * renderer).
 */
const suites: Record<string, string[]> = {
  pie: ['pie_spec', 'pie_grammar'],
  er: [
    'er_spec',
    'er_subgraph_spec',
    'er_db_spec',
    'er_characters_spec',
    'render:er_get_data_spec',
  ],
  gantt: ['gantt_parser_spec', 'gantt_db_spec', 'render:gantt_render_spec'],
  class: [
    'class_parser_spec',
    'class_definition_spec',
    'class_members_generics_spec',
    'class_relationships_spec',
    'class_labels_spec',
    'class_styles_spec',
    'class_brackets_comments_clicks_annotations_spec',
    'render:class_db_get_data_spec',
    'render:class_types_display_spec',
  ],
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
    'render:flowchart_arrow_types_spec',
    'render:flowchart_db_get_data_spec',
    'render:flowchart_edge_marker_spec',
    'render:flowchart_edges_spec',
    'render:flowchart_line_jump_spec',
    'render:flowchart_markers_spec',
    'render:flowchart_node_data_get_data_spec',
    'render:flowchart_subgraph_get_data_spec',
  ],
  sequence: [
    'sequence_parse_spec',
    'sequence_statements_spec',
    'sequence_db_spec',
    'sequence_keyword_actors_spec',
    'render:sequence_render_spec',
  ],
  state: [
    'state_parser_spec',
    'state_db_spec',
    'state_style_spec',
    'state_diagram_v2_spec',
    'state_diagram_spec',
    'state_renderer_spec',
    'render:state_data_color_index_spec',
    'render:state_data_note_spec',
    'render:state_data_style_spec',
  ],
  mindmap: ['mindmap_spec', 'mindmap_colors_spec', 'mindmap_get_data_spec', 'mindmap_layout_spec'],
  xychart: ['xychart_parser_spec', 'xychart_db_spec'],
  quadrantChart: ['quadrant_chart_parser_spec', 'quadrant_chart_db_spec'],
};

interface Counts {
  ported: number;
  ignored: number;
  passing: number;
}

function countSuite(suite: string): Counts {
  // Cargo's own summary counts the tests that the macros generate, which a textual count misses.
  const [crate, name] = suite.startsWith('render:')
    ? ['ferrigura-render', suite.slice('render:'.length)]
    : ['ferrigura-syntax', suite];
  const run = spawnSync('cargo', ['test', '--quiet', '-p', crate, '--test', name], {
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

/**
 * The differential cell of a run, or null when this checkout has no summary of it. `name` is the
 * run's name: the type for a parse (`flowchart`), `<type>-data` for the layout data.
 */
async function differentialCell(name: string): Promise<string | null> {
  try {
    const summary = JSON.parse(
      await readFile(`${root}target/port/${name}-summary.json`, 'utf8')
    ) as { inputs: number; differences: number; notYet?: number };
    const notYet = summary.notYet ? `, ${summary.notYet} not yet ported` : '';
    return summary.differences === 0
      ? `${summary.inputs} inputs, no difference${notYet}`
      : `${summary.differences} of ${summary.inputs} differ${notYet}`;
  } catch {
    return null;
  }
}

/**
 * The geometry cell of a type, or null when this checkout has no summary of a run. The compared
 * fixtures and probes are those within or outside the tolerance and those ferrigura does not render;
 * the excluded ones (`look: handDrawn`, scripts no bundled face covers) and those the reference
 * fails on are not compared.
 */
async function geometryCell(type: string): Promise<string | null> {
  try {
    const { summary } = JSON.parse(
      await readFile(`${root}target/geometry/generic-${type}-geometry.json`, 'utf8')
    ) as {
      summary: Record<
        'within' | 'outside' | 'not rendered' | 'excluded' | 'reference failed',
        number
      >;
    };
    const compared = summary.within + summary.outside + summary['not rendered'];
    const notCompared = summary.excluded + summary['reference failed'];
    return (
      `${summary.within} of ${compared} within tolerance` +
      (notCompared > 0 ? ` (${notCompared} not compared)` : '')
    );
  } catch {
    return null;
  }
}

/** The types whose layout data (`getData()`) has a differential run of its own, `<type>-data`. */
const layoutDataTypes = new Set(['flowchart', 'class', 'state', 'er']);

/** The page of each type under `docs/diagrams/`, which names the type's row in the table. */
const pages: Record<string, string> = {
  pie: 'pie',
  er: 'er',
  gantt: 'gantt',
  class: 'class',
  flowchart: 'flowchart',
  sequence: 'sequence',
  state: 'state',
  mindmap: 'mindmap',
  xychart: 'xychart',
  quadrantChart: 'quadrant',
};

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

const table = await readFile(tablePath, 'utf8');
const differential: Record<string, string | null> = {};
const layoutData: Record<string, string | null> = {};
const geometry: Record<string, string | null> = {};
for (const type of Object.keys(suites)) {
  differential[type] = await differentialCell(type);
  layoutData[type] = layoutDataTypes.has(type) ? await differentialCell(`${type}-data`) : null;
  geometry[type] = await geometryCell(type);
}

/**
 * Rewrites the cells of one row of the coverage table. The cells of the row are the type (a link to
 * its page), its keywords, its layout engine, the specs (passing / ported, ignored), the
 * differential parse, the layout data and the geometry.
 */
function rewriteRow(line: string, type: string): string {
  const cells = line.split('|');
  const { ported, ignored, passing } = counts[type];
  cells[4] = ` ${passing} / ${ported}${ignored > 0 ? ` (${ignored} ignored)` : ''} `;
  // Without a summary of a run here, a cell keeps what an earlier run wrote.
  if (differential[type] !== null) cells[5] = ` ${differential[type]} `;
  if (layoutData[type] !== null) cells[6] = ` ${layoutData[type]} `;
  if (geometry[type] !== null) cells[7] = ` ${geometry[type]} `;
  return cells.join('|');
}

const rewritten = table
  .split('\n')
  .map((line) => {
    for (const type of Object.keys(suites)) {
      // The type's cell is a link to its page, padded to the width of the column.
      if (
        line.startsWith('| [') &&
        new RegExp(`^\\| \\[[^\\]]+\\]\\(\\./${pages[type]}\\.md\\)\\s*\\|`).test(line)
      ) {
        return rewriteRow(line, type);
      }
    }
    return line;
  })
  .join('\n');

await writeFile(tablePath, rewritten);
// The table's column widths follow the formatter, so the file is formatted after the rewrite.
spawnSync('pnpm', ['-C', 'tools/port', 'exec', 'tsx', 'format.ts', tablePath], {
  cwd: root,
  stdio: 'ignore',
});
for (const [type, count] of Object.entries(counts)) {
  console.log(
    `${type}: ${count.passing} passing of ${count.ported} ported, ${count.ignored} ignored; ` +
      `differential: ${differential[type] ?? 'no summary here, cell kept'}; ` +
      `layout data: ${layoutData[type] ?? 'kept'}; geometry: ${geometry[type] ?? 'kept'}`
  );
}
