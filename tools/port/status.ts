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

/**
 * The ported suites of each diagram type, as file names in `crates/ferrigura-syntax/tests/`, or in
 * `crates/ferrigura-render/tests/` with the prefix `render:` (the specs of the layout data and the
 * renderer).
 */
const suites: Record<string, string[]> = {
  pie: ['pie_spec', 'pie_grammar'],
  timeline: ['timeline_spec', 'timeline_grammar', 'render:timeline_render'],
  info: ['info_spec', 'info_grammar'],
  journey: ['journey_spec', 'journey_db_spec'],
  er: [
    'er_spec',
    'er_subgraph_spec',
    'er_db_spec',
    'er_characters_spec',
    'render:er_get_data_spec',
  ],
  gantt: ['gantt_parser_spec', 'gantt_db_spec', 'render:gantt_render_spec'],
  gitgraph: ['git_graph_spec', 'git_graph_parser', 'git_graph_populate'],
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
    'render:flowchart_line_jump_integration_spec',
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
  // The P2 types have no suites until their parser tasks add them. Their rows of the P2 table are
  // left as written while the list is empty.
  mindmap: ['mindmap_spec', 'mindmap_colors_spec', 'mindmap_get_data_spec', 'mindmap_layout_spec'],
  block: ['block_spec', 'block_db_spec', 'block_layout_spec', 'block_color_index_spec'],
  packet: [],
  xychart: ['xychart_parser_spec', 'xychart_db_spec'],
  sankey: [],
  quadrantChart: [],
  requirement: ['requirement_spec', 'requirement_db_spec'],
  c4: [
    'c4_diagram_spec',
    'c4_label_spec',
    'c4_boundary_spec',
    'c4_component_spec',
    'c4_container_spec',
    'c4_named_attributes_spec',
    'c4_person_spec',
    'c4_person_ext_spec',
    'c4_system_spec',
  ],
  kanban: ['kanban_spec', 'kanban_db_spec', 'kanban_quirks_spec'],
  architecture: [],
  treemap: [],
  ishikawa: [],
  eventmodeling: ['event_modeling_spec', 'event_modeling_grammar', 'event_modeling_validator'],
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

/** The name of a type's row in the status tables, where it is not the type itself. */
const rowNames: Record<string, string> = { er: 'ER', gitgraph: 'gitGraph', c4: 'C4' };

/** Rewrites the parser, specs and differential cells of one row of a type with suites. */
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
      if (suites[type].length > 0 && line.startsWith(`| ${rowNames[type] ?? type} `)) {
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
