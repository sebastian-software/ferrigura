/**
 * Named exceptions of the differential runs against the reference, upstream Mermaid at the pin
 * (docs-port/ORACLE-PLAN.md). Each entry names an input by its text, says why it differs, and is
 * one of two kinds:
 *
 * - `pending`: ferrigura is behind upstream, mostly after a move of the pin. The entry names the
 *   issue that tracks the fix and goes with it.
 * - `unsupported`: ferrigura does not do this on purpose; the reason says what instead.
 *
 * A bug of Mermaid is not an entry: its fix is a patch on the pinned checkout (tools/reference). The
 * runs check every entry: an entry that now agrees is an error, so the list cannot outlive the
 * difference it records, and a pending entry without an issue is an error too.
 */
export interface PortException {
  /** The exact text of the input. */
  text: string;
  /** Why the outputs differ. */
  reason: string;
  /** `pending` (ferrigura is behind upstream) or `unsupported` (on purpose). */
  kind: 'pending' | 'unsupported';
  /** The issue that tracks a pending entry, as `#123` or its URL. */
  issue?: string;
}

export const exceptions: Record<string, PortException[]> = {
  config: [],
  flowchart: [],
  class: [],
  'flowchart-data': [],
  'state-data': [],
  timeline: [],
  journey: [],
  gitgraph: [],
  er: [],
  sequence: [],
  state: [],
  gantt: [],
  mindmap: [],
  block: [],
  packet: [],
  xychart: [],
  sankey: [],
  quadrantChart: [],
  requirement: [],
  'requirement-data': [],
  c4: [],
  kanban: [],
  architecture: [],
  treemap: [],
  ishikawa: [],
  eventmodeling: [],
  agentflow: [],
  cynefin: [],
  radar: [],
  railroad: [],
  swimlanes: [],
  treeView: [],
  usecase: [],
  venn: [],
  wardley: [],
};

// The runs that take exceptions import this file, so a pending entry without an issue stops them.
const withoutIssue = Object.entries(exceptions).flatMap(([run, entries]) =>
  entries
    .filter((entry) => entry.kind === 'pending' && !entry.issue)
    .map((entry) => `${run}: a pending exception names no issue: ${JSON.stringify(entry.text)}`)
);
if (withoutIssue.length > 0) {
  throw new Error(withoutIssue.join('\n'));
}
