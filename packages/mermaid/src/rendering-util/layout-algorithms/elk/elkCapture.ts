// ─────────────────────────────────────────────────────────────────────────────
// ELK layout capture (dev / test tooling — NOT part of production rendering)
//
// `runElkLayout` in render.ts dynamically imports this module ONLY when
// `globalThis.mermaidCaptureElk` is truthy, so production renders pay for a
// single property read and never load this code on the hot path.
//
// When enabled, every elkjs call appends one entry to
// `globalThis.mermaidElkCaptures` (an array, created on first use):
//
//   { version, index, algorithm, input, output }
//
// - `input`: the graph as elkjs received it. render.ts snapshots it with a JSON
//   round-trip BEFORE `elk.layout` runs, because elkjs may mutate its argument.
// - `output`: the JSON round-trip of the result elkjs returned.
// - `algorithm`: the root `elk.algorithm` layout option, or null when unset.
// - `index`: 0-based position of this call among the captures of this page.
//
// Consumed by the corpus generator in the ELK port crate's tools folder
// (capture-corpus.ts), which sets the flag before rendering each fixture and
// re-runs `input` through Node elkjs.
// ─────────────────────────────────────────────────────────────────────────────

/** Version of the capture entry shape. Bump when `ElkCaptureEntry` changes. */
export const ELK_CAPTURE_VERSION = 1;

export interface ElkCaptureEntry {
  version: number;
  index: number;
  algorithm: string | null;
  input: unknown;
  output: unknown;
}

interface ElkCaptureGlobal {
  mermaidCaptureElk?: boolean;
  mermaidElkCaptures?: ElkCaptureEntry[];
}

export interface ElkCaptureCall {
  /** Snapshot of the graph taken before elkjs ran (already a JSON round-trip). */
  input: unknown;
  /** The result elkjs returned. Round-tripped here, not mutated. */
  output: unknown;
  /** Root `elk.algorithm` layout option of the graph. */
  algorithm: string | undefined;
}

/**
 * Append one captured elkjs call to `globalThis.mermaidElkCaptures`.
 *
 * Called only from render.ts's `runElkLayout` when `globalThis.mermaidCaptureElk`
 * is set. Because elkjs never sees this module, the layout result is unchanged.
 */
export function recordElkCapture(call: ElkCaptureCall): void {
  const g = globalThis as unknown as ElkCaptureGlobal;
  // `mermaidElkCaptures` accumulates for the life of the page. Reload the page,
  // or `delete globalThis.mermaidElkCaptures`, to start over.
  g.mermaidElkCaptures ??= [];
  g.mermaidElkCaptures.push({
    version: ELK_CAPTURE_VERSION,
    index: g.mermaidElkCaptures.length,
    algorithm: call.algorithm ?? null,
    input: call.input,
    output: JSON.parse(JSON.stringify(call.output)),
  });
}
