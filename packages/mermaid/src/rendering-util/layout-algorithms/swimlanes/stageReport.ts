import type { LayoutData } from '../../types.js';

/**
 * Stage reports of the swimlane layout core and its post-processing (dev and test tooling only).
 *
 * A page that sets `globalThis.__mermaidSwimlaneStage` gets one call per stage with a JSON copy of
 * the data, so a report cannot change the layout. Without the hook nothing is copied. The capture
 * tools that install it are in `crates/ferrigura-swimlane/tools/`.
 */
export type SwimlaneStageHook = (stage: string, data: unknown) => void;

/** The installed hook, or undefined when the page has none. */
export function swimlaneStageHook(): SwimlaneStageHook | undefined {
  const hook = (globalThis as typeof globalThis & { __mermaidSwimlaneStage?: SwimlaneStageHook })
    .__mermaidSwimlaneStage;
  return typeof hook === 'function' ? hook : undefined;
}

/** Reports one stage. `data` is evaluated only when a hook is installed. */
export function reportSwimlaneStage(stage: string, data: () => unknown): void {
  const hook = swimlaneStageHook();
  if (hook !== undefined) {
    hook(stage, data());
  }
}

const NON_FINITE_MARK = '__nonFinite__:';

/**
 * A JSON copy of a value. Map entries and Set members become arrays in insertion order, keys whose
 * value is undefined or a function are dropped, and a non-finite number becomes a string that starts
 * with `__nonFinite__:`, so that the capture can refuse it (JSON would turn it into null).
 */
export function copyForStage(value: unknown): unknown {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : `${NON_FINITE_MARK}${String(value)}`;
  }
  if (value instanceof Map) {
    return Array.from(value, ([key, item]) => [copyForStage(key), copyForStage(item)]);
  }
  if (value instanceof Set) {
    return Array.from(value, (item: unknown) => copyForStage(item));
  }
  if (Array.isArray(value)) {
    return value.map((item: unknown) => copyForStage(item));
  }
  if (typeof value === 'object' && value !== null) {
    const copy: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value)) {
      if (item !== undefined && typeof item !== 'function') {
        copy[key] = copyForStage(item);
      }
    }
    return copy;
  }
  return value;
}

/** The layout fields the stages report, as JSON copies: `direction`, `config`, `nodes` and `edges`. */
export function copyLayoutForStage(layout: LayoutData): unknown {
  return copyForStage({
    direction: (layout as LayoutData & { direction?: string }).direction ?? null,
    config: layout.config,
    nodes: layout.nodes ?? [],
    edges: layout.edges ?? [],
  });
}

/** Collects the post-processing passes of one call and reports them in run order. */
export interface PassReporter {
  /** Notes that a pass has just run. The copy of the layout is taken now. */
  pass(name: string): void;
  /** Reports the collected passes. */
  flush(): void;
}

/**
 * The stage of each post-processing pass is `post:<name>`. A pass that runs more than once gets
 * `#k` on each of its runs, k counting the runs in order. Whether a pass repeats is known only when
 * the post-processing has finished, so the copies are taken as the passes run and the reports are
 * made at the end, in the same order.
 */
export function createPassReporter(layout: LayoutData): PassReporter {
  const hook = swimlaneStageHook();
  if (hook === undefined) {
    return { pass: () => undefined, flush: () => undefined };
  }
  const runs: { name: string; data: unknown }[] = [];
  return {
    pass: (name) => {
      runs.push({ name, data: copyLayoutForStage(layout) });
    },
    flush: () => {
      const totals = new Map<string, number>();
      for (const run of runs) {
        totals.set(run.name, (totals.get(run.name) ?? 0) + 1);
      }
      const seen = new Map<string, number>();
      for (const run of runs) {
        const k = (seen.get(run.name) ?? 0) + 1;
        seen.set(run.name, k);
        const suffix = (totals.get(run.name) ?? 0) > 1 ? `#${k}` : '';
        hook(`post:${run.name}${suffix}`, run.data);
      }
    },
  };
}
