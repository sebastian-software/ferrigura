/**
 * Named exceptions of the differential runs against the reference, upstream Mermaid at the pin
 * (docs/decisions/0014-upstream-reference-at-a-pin.md). Each entry names an input by its text, says why it differs, and is
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
  /** Optional substring of ferrigura's expected error when the reference accepts the input. */
  expectedError?: string;
}

export const exceptions: Record<string, PortException[]> = {
  config: [],
  flowchart: [],
  class: [],
  'flowchart-data': [],
  'state-data': [],
  er: [],
  sequence: [],
  state: [],
  gantt: [],
  mindmap: [],
  xychart: [],
  quadrantChart: [],
  yaml: [
    {
      text: [
        'a0: &a0 [1, 1]',
        ...Array.from({ length: 14 }, (_, i) => `a${i + 1}: &a${i + 1} [*a${i}, *a${i}]`),
      ].join('\n'),
      kind: 'unsupported',
      reason:
        'ferrigura bounds YAML alias expansion to prevent Billion Laughs memory exhaustion; js-yaml shares JS object references without materializing the tree',
      expectedError: 'YAML alias expansion exceeded size limit',
    },
    {
      text: [
        `s: &s "${'x'.repeat(40_000)}"`,
        'a: &a [*s, *s, *s, *s]',
        'b: &b [*a, *a, *a, *a]',
        'c: [*b, *b, *b, *b]',
      ].join('\n'),
      kind: 'unsupported',
      reason:
        'ferrigura bounds expanded YAML string bytes to prevent alias memory amplification; js-yaml shares JS string references without copying',
      expectedError: 'YAML alias expansion exceeded size limit',
    },
    {
      text: [
        'a0: &a0 [1]',
        ...Array.from({ length: 250 }, (_, i) => `a${i + 1}: &a${i + 1} [*a${i}]`),
      ].join('\n'),
      kind: 'unsupported',
      reason:
        'ferrigura bounds expanded YAML alias nesting depth to prevent stack overflow during tree conversion; js-yaml shares JS object references without deep expansion',
      expectedError: 'YAML alias expansion exceeded nesting limit',
    },
  ],
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
