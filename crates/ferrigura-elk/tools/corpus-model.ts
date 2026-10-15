/**
 * Pure helpers for the ELK differential corpus (see capture-corpus.ts).
 *
 * Nothing here touches the browser, the filesystem or elkjs, so the functions
 * can be reasoned about on their own.
 */

/** Corpus case file version. Bump when the shape of a case file changes. */
export const CORPUS_VERSION = 1;

/** A JSON value as it comes out of `JSON.parse`. */
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

/** Keys elkjs (GWT) adds to every object for identity hashing. They carry no layout meaning. */
const GWT_HASH_KEY = '$H';

/** Deep copy of a JSON-compatible value. Matches what the capture hook does to its input. */
export function jsonRoundTrip<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/**
 * Return a copy of `value` with every `$H` key removed, at any depth.
 *
 * Arrays and plain objects are copied; other values are returned as they are.
 */
export function stripGwtHashKeys(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => stripGwtHashKeys(item));
  }
  if (value !== null && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [key, child] of Object.entries(value)) {
      if (key !== GWT_HASH_KEY) {
        out[key] = stripGwtHashKeys(child);
      }
    }
    return out;
  }
  return value;
}

/**
 * Describe the first place two JSON values differ, as a JSON-path-ish string.
 *
 * Returns `undefined` when they are deep-equal. Both sides should already be
 * normalised (JSON round-tripped, `$H` stripped).
 */
export function firstDifference(a: unknown, b: unknown, path = '$'): string | undefined {
  if (Object.is(a, b)) {
    return undefined;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) {
      return `${path}: array length ${a.length} vs ${b.length}`;
    }
    for (const [i, element] of a.entries()) {
      const diff = firstDifference(element, b[i], `${path}[${i}]`);
      if (diff) {
        return diff;
      }
    }
    return undefined;
  }
  if (isPlainObject(a) && isPlainObject(b)) {
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
    for (const key of keys) {
      if (!(key in a)) {
        return `${path}.${key}: missing on browser side`;
      }
      if (!(key in b)) {
        return `${path}.${key}: missing on node side`;
      }
      const diff = firstDifference(a[key], b[key], `${path}.${key}`);
      if (diff) {
        return diff;
      }
    }
    return undefined;
  }
  return `${path}: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export interface GraphStats {
  /** Nodes below the root, at any depth. Ports and labels are not counted. */
  nodes: number;
  /** Edges in the root and in every container, at any depth. */
  edges: number;
  /** True when some node has child nodes of its own. */
  compound: boolean;
  /** True when some edge carries at least one label. */
  edgeLabels: boolean;
}

/** Count the shape of an ELK graph (the `input` of a case). */
export function graphStats(root: Record<string, unknown>): GraphStats {
  const stats: GraphStats = { nodes: 0, edges: 0, compound: false, edgeLabels: false };
  const walk = (container: Record<string, unknown>): void => {
    for (const edge of asArray(container.edges)) {
      stats.edges++;
      if (asArray(edge.labels).length > 0) {
        stats.edgeLabels = true;
      }
    }
    for (const child of asArray(container.children)) {
      stats.nodes++;
      if (asArray(child.children).length > 0) {
        stats.compound = true;
      }
      walk(child);
    }
  };
  walk(root);
  return stats;
}

function asArray(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? (value as Record<string, unknown>[]) : [];
}

/** File-name-safe, lowercase, hyphen-separated form of a fixture stem. */
export function slugify(stem: string): string {
  return stem
    .toLowerCase()
    .replace(/[^\da-z]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/**
 * Root layout options Mermaid's `createRootElkGraph` sets for an ELK layered run
 * with the default config, copied from render.ts (the `default` preset and the
 * option lists that are empty in production).
 *
 * Options that Mermaid sets to `undefined` when the config leaves them unset
 * (`elk.layered.mergeEdges`, `elk.layered.crossingMinimization.forceNodeModelOrder`,
 * `elk.layered.considerModelOrder.strategy`, `elk.layered.layering.coffmanGraham.layerBound`)
 * are left out: JSON drops them, so a captured input never carries them either.
 */
export function rootLayoutOptionsDefault(): Record<string, string | number | boolean> {
  return {
    'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
    'elk.algorithm': 'elk.layered',
    'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
    'elk.layered.nodePlacement.bk.fixedAlignment': 'BALANCED',
    'elk.direction': 'DOWN',
    'spacing.baseValue': 40,
    'elk.layered.unnecessaryBendpoints': true,
    'elk.layered.cycleBreaking.strategy': 'DEPTH_FIRST',
    'elk.layered.layering.strategy': 'NETWORK_SIMPLEX',
    'elk.layered.wrapping.multiEdge.improveCuts': true,
    'elk.layered.wrapping.multiEdge.improveWrappedEdges': true,
    'elk.layered.edgeRouting.selfLoopDistribution': 'EQUALLY',
    'elk.layered.mergeHierarchyEdges': true,
    'elk.spacing.portsSurrounding': '[top=12,left=12,bottom=12,right=12]',
  };
}

/** A root graph with `count` leaf nodes (50 x 40) and no edges, using Mermaid's root options. */
export function nodesOnlyGraph(count: number): Record<string, unknown> {
  return {
    id: 'root',
    layoutOptions: rootLayoutOptionsDefault(),
    children: Array.from({ length: count }, (_, i) => ({
      id: `n${i}`,
      width: 50,
      height: 40,
    })),
    edges: [],
  };
}
