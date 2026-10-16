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

/**
 * A separately laid out child with two connected components: a chain of two nodes and a second
 * chain, in a child that is laid out on its own. The edges are written at the root, as Mermaid
 * writes them, so their containment is the child, where the root's edges meet the child's nodes. No
 * layer of a component has two nodes, so the greedy switch does nothing in either.
 */
export function syntheticComponentGraphs(): { name: string; graph: Record<string, unknown> }[] {
  const node = (id: string): Record<string, unknown> => ({ id, width: 50, height: 40 });
  const edge = (id: string, source: string, target: string): Record<string, unknown> => ({
    id,
    sources: [source],
    targets: [target],
  });
  return [
    {
      name: 'synthetic__components-not-separated',
      graph: {
        id: 'root',
        layoutOptions: {
          ...rootLayoutOptionsDefault(),
          'elk.separateConnectedComponents': false,
        },
        children: [node('a'), node('b'), node('c'), node('d')],
        edges: [edge('e0', 'a', 'b'), edge('e1', 'c', 'd')],
      },
    },
    {
      name: 'synthetic__nested-two-components',
      graph: {
        id: 'root',
        layoutOptions: rootLayoutOptionsDefault(),
        children: [
          {
            id: 'x',
            layoutOptions: {
              'elk.algorithm': 'elk.layered',
              'elk.direction': 'DOWN',
              'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
              // As Mermaid's composite states set it.
              'elk.layered.cycleBreaking.strategy': 'DEPTH_FIRST',
            },
            children: [node('a'), node('b'), node('c'), node('d')],
            edges: [],
          },
        ],
        edges: [edge('e0', 'a', 'b'), edge('e1', 'c', 'd')],
      },
    },
  ];
}

/**
 * The fan of `syntheticEdgeGraphs` with labels on some of its edges, under each model order
 * strategy. The labels add dummies to the layers, and the model order decides the order of the
 * ports and nodes that the greedy switch then works on.
 */
export function syntheticModelOrderLabelGraphs(): {
  name: string;
  graph: Record<string, unknown>;
}[] {
  const ids = ['n0', 'n1', 'n2', 'n3', 'n4'];
  const pairs: [string, string][] = [
    ['n0', 'n1'],
    ['n0', 'n2'],
    ['n0', 'n3'],
    ['n1', 'n4'],
    ['n2', 'n4'],
    ['n3', 'n4'],
    ['n0', 'n4'],
    ['n0', 'n4'],
  ];
  const labelled = new Set([0, 3, 6]);
  const graph = (strategy: string): Record<string, unknown> => ({
    id: 'root',
    layoutOptions: {
      ...rootLayoutOptionsDefault(),
      'elk.layered.considerModelOrder.strategy': strategy,
    },
    children: ids.map((id) => ({ id, width: 50, height: 40 })),
    edges: pairs.map(([source, target], index) => ({
      id: `e${index}`,
      sources: [source],
      targets: [target],
      ...(labelled.has(index)
        ? {
            labels: [
              {
                text: `l${index}`,
                width: 20 + index,
                height: 10,
                layoutOptions: { 'elk.edgeLabels.placement': 'CENTER' },
              },
            ],
          }
        : {}),
    })),
  });
  return ['NONE', 'NODES_AND_EDGES', 'PREFER_EDGES', 'PREFER_NODES'].map((strategy) => ({
    name: `synthetic__model-order-labels-${strategy.toLowerCase().replace(/_/g, '-')}`,
    graph: graph(strategy),
  }));
}

/**
 * Two nodes in each of two layers, with three edges from one node and one from the other, under the
 * model order strategies that compare ports by their edges (NODES_AND_EDGES, PREFER_EDGES). The
 * sweep takes its first attempt with the initial order, and the random seed decides the next
 * attempts: seed 3 gives a different layout from the one elkjs gives under seed 1.
 */
export function syntheticModelOrderFirstTryGraphs(): {
  name: string;
  graph: Record<string, unknown>;
}[] {
  const graph = (strategy: string): Record<string, unknown> => ({
    id: 'root',
    layoutOptions: {
      'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
      'elk.algorithm': 'elk.layered',
      'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
      'elk.layered.nodePlacement.bk.fixedAlignment': 'BALANCED',
      'elk.direction': 'RIGHT',
      'spacing.baseValue': 40,
      'elk.layered.unnecessaryBendpoints': true,
      'elk.layered.cycleBreaking.strategy': 'DEPTH_FIRST',
      'elk.layered.layering.strategy': 'NETWORK_SIMPLEX',
      'elk.layered.mergeEdges': false,
      'elk.randomSeed': 3,
      'elk.layered.considerModelOrder.strategy': strategy,
    },
    children: [
      { id: 'n1', width: 57, height: 55 },
      { id: 'n2', width: 50, height: 40 },
      { id: 'n3', width: 43, height: 47 },
      { id: 'n4', width: 46, height: 26 },
    ],
    edges: [
      { id: 'e3', sources: ['n3'], targets: ['n4'] },
      { id: 'e4', sources: ['n2'], targets: ['n4'] },
      { id: 'e5', sources: ['n2'], targets: ['n4'] },
      { id: 'e7', sources: ['n2'], targets: ['n1'] },
    ],
  });
  return [
    { name: 'synthetic__model-order-first-try-nodes-and-edges', graph: graph('NODES_AND_EDGES') },
    { name: 'synthetic__model-order-first-try-prefer-edges', graph: graph('PREFER_EDGES') },
  ];
}

/**
 * A flat graph whose layer sweep ends with marked edges, so the Brandes & Koepf log names its edges.
 * The root's edges are named with the root's id in front (`e_root.e2`), as elkjs writes them.
 */
export function syntheticMarkedEdgeGraphs(): { name: string; graph: Record<string, unknown> }[] {
  return [
    {
      name: 'synthetic__marked-edges-root-names',
      graph: {
        id: 'root',
        layoutOptions: {
          'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
          'elk.algorithm': 'elk.layered',
          'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
          'elk.layered.nodePlacement.bk.fixedAlignment': 'BALANCED',
          'elk.direction': 'RIGHT',
          'spacing.baseValue': 40,
          'elk.layered.unnecessaryBendpoints': true,
          'elk.layered.cycleBreaking.strategy': 'DEPTH_FIRST',
          'elk.layered.layering.strategy': 'NETWORK_SIMPLEX',
          'elk.layered.mergeEdges': false,
          'elk.randomSeed': 44,
        },
        children: [
          { id: 'n0', width: 54, height: 38 },
          { id: 'n1', width: 78, height: 20 },
          { id: 'n2', width: 35, height: 57 },
          { id: 'n3', width: 24, height: 51 },
          { id: 'n5', width: 25, height: 47 },
        ],
        edges: [
          { id: 'e2', sources: ['n2'], targets: ['n3'] },
          { id: 'e3', sources: ['n1'], targets: ['n3'] },
          { id: 'e4', sources: ['n1'], targets: ['n5'] },
          { id: 'e5', sources: ['n3'], targets: ['n5'] },
          { id: 'e6', sources: ['n1'], targets: ['n5'] },
          { id: 'e7', sources: ['n1'], targets: ['n2'] },
          { id: 'e8', sources: ['n2'], targets: ['n0'] },
        ],
      },
    },
  ];
}

/**
 * A flat graph with cycles, laid out with the greedy cycle breaker, which Mermaid's legacy preset
 * selects through the GREEDY strategy. The flows of its nodes differ, so the sinks, the sources and
 * the random draw of the node with the largest outflow all take part.
 */
export function syntheticGreedyCycleGraphs(): { name: string; graph: Record<string, unknown> }[] {
  const node = (id: string): Record<string, unknown> => ({ id, width: 50, height: 40 });
  const edge = (id: string, source: string, target: string): Record<string, unknown> => ({
    id,
    sources: [source],
    targets: [target],
  });
  return [
    {
      name: 'synthetic__greedy-cycles',
      graph: {
        id: 'root',
        layoutOptions: {
          ...rootLayoutOptionsDefault(),
          'elk.layered.cycleBreaking.strategy': 'GREEDY',
        },
        children: [node('a'), node('b'), node('c'), node('d'), node('e')],
        edges: [
          edge('e0', 'a', 'b'),
          edge('e1', 'b', 'c'),
          edge('e2', 'c', 'a'),
          edge('e3', 'c', 'd'),
          edge('e4', 'd', 'e'),
          edge('e5', 'e', 'b'),
          edge('e6', 'a', 'd'),
        ],
      },
    },
  ];
}

/**
 * Flat graphs whose leaf nodes carry labels, for the node label sizing of the port: the inside,
 * top-centred labels that the nested state diagrams use, with the NODE_LABELS and MINIMUM_SIZE
 * constraints. The labels are wider than their nodes in one case, and the node has a minimum width
 * in another. There are no edges, so only the node sizes and label boxes are exercised.
 */
export function syntheticNodeLabelGraphs(): { name: string; graph: Record<string, unknown> }[] {
  const labelled = (
    id: string,
    width: number,
    height: number,
    text: string,
    labelWidth: number,
    options: Record<string, string> = {}
  ): Record<string, unknown> => ({
    id,
    width,
    height,
    layoutOptions: {
      'nodeLabels.placement': '[H_CENTER V_TOP, INSIDE]',
      'nodeSize.constraints': '[MINIMUM_SIZE, NODE_LABELS]',
      ...options,
    },
    labels: [{ text, width: labelWidth, height: 19 }],
  });
  return [
    {
      name: 'synthetic__node-labels-inside-top',
      graph: {
        id: 'root',
        layoutOptions: rootLayoutOptionsDefault(),
        children: [
          labelled('a', 50, 40, 'A', 12),
          labelled('b', 30, 30, 'Wide label', 80, { 'nodeSize.minimum': '(120, 0)' }),
          {
            id: 'c',
            width: 50,
            height: 40,
            labels: [{ text: 'C', width: 12, height: 19 }],
          },
        ],
        edges: [],
      },
    },
  ];
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

/**
 * Hand-built graphs with edges, for the steps before label dummies: a chain, a diamond (fan-out,
 * then fan-in), a cycle, and a long edge that skips a layer. Nodes are 50 by 40, and the root has
 * Mermaid's options. The self loop graphs of `syntheticSelfLoopGraphs` come last, with labels.
 */
export function syntheticEdgeGraphs(): { name: string; graph: Record<string, unknown> }[] {
  const graph = (ids: string[], pairs: [string, string][]): Record<string, unknown> => ({
    id: 'root',
    layoutOptions: rootLayoutOptionsDefault(),
    children: ids.map((id) => ({ id, width: 50, height: 40 })),
    edges: pairs.map(([source, target], index) => ({
      id: `e${index}`,
      sources: [source],
      targets: [target],
    })),
  });
  // A fan-out, a fan-in, parallel long edges through dummies, under the two ordering strategies that
  // compare ports and nodes by their edges (PREFER_EDGES) or by their nodes (PREFER_NODES).
  const fan = graph(
    ['n0', 'n1', 'n2', 'n3', 'n4'],
    [
      ['n0', 'n1'],
      ['n0', 'n2'],
      ['n0', 'n3'],
      ['n1', 'n4'],
      ['n2', 'n4'],
      ['n3', 'n4'],
      ['n0', 'n4'],
      ['n0', 'n4'],
    ]
  );
  const ordered = (strategy: string): Record<string, unknown> => ({
    ...fan,
    layoutOptions: {
      ...rootLayoutOptionsDefault(),
      'elk.layered.considerModelOrder.strategy': strategy,
    },
  });
  return [
    { name: 'synthetic__edges-prefer-edges', graph: ordered('PREFER_EDGES') },
    { name: 'synthetic__edges-prefer-nodes', graph: ordered('PREFER_NODES') },
    {
      name: 'synthetic__edges-chain-3',
      graph: graph(
        ['n0', 'n1', 'n2'],
        [
          ['n0', 'n1'],
          ['n1', 'n2'],
        ]
      ),
    },
    {
      name: 'synthetic__edges-diamond-4',
      graph: graph(
        ['n0', 'n1', 'n2', 'n3'],
        [
          ['n0', 'n1'],
          ['n0', 'n2'],
          ['n1', 'n3'],
          ['n2', 'n3'],
        ]
      ),
    },
    {
      name: 'synthetic__edges-cycle-3',
      graph: graph(
        ['n0', 'n1', 'n2'],
        [
          ['n0', 'n1'],
          ['n1', 'n2'],
          ['n2', 'n0'],
        ]
      ),
    },
    {
      name: 'synthetic__edges-long-3',
      graph: graph(
        ['n0', 'n1', 'n2'],
        [
          ['n0', 'n1'],
          ['n1', 'n2'],
          ['n0', 'n2'],
        ]
      ),
    },
    ...syntheticSelfLoopGraphs(),
  ];
}

/**
 * Hand-built graphs with self loops, one per branch of the self loop processors: both orderings,
 * the distributions that put loops on one side and on two sides, the corner loops, stacked labels,
 * and opposing loops, which need fixed ports. Each self loop has two ports of its own, as the
 * importer makes them for an edge from a node to itself.
 *
 * Three- and four-side loops are left out. A loop on three or four sides takes several edges
 * through one shared port, which is a hyper loop, and hyper loops are not ported.
 */
function syntheticSelfLoopGraphs(): { name: string; graph: Record<string, unknown> }[] {
  const plainNode = (id: string): Record<string, unknown> => ({ id, width: 50, height: 40 });
  const loopNode = (
    id: string,
    options: Record<string, string>,
    extra: Record<string, unknown> = {}
  ): Record<string, unknown> => ({ id, width: 50, height: 40, layoutOptions: options, ...extra });
  const port = (id: string, side: string): Record<string, unknown> => ({
    id,
    width: 0,
    height: 0,
    layoutOptions: { 'elk.port.side': side },
  });
  // An edge between two ends (nodes or ports), with a centred label for each text.
  const edge = (
    id: string,
    source: string,
    target: string,
    labels: string[] = []
  ): Record<string, unknown> => ({
    id,
    sources: [source],
    targets: [target],
    ...(labels.length > 0
      ? {
          labels: labels.map((text) => ({
            text,
            width: 7 * text.length,
            height: 12,
            layoutOptions: { 'edgeLabels.placement': 'CENTER' },
          })),
        }
      : {}),
  });
  const root = (
    children: Record<string, unknown>[],
    edges: Record<string, unknown>[]
  ): Record<string, unknown> => ({
    id: 'root',
    layoutOptions: rootLayoutOptionsDefault(),
    children,
    edges,
  });
  const north = 'elk.layered.edgeRouting.selfLoopDistribution';
  const ordering = 'elk.layered.edgeRouting.selfLoopOrdering';
  // Eight loops on one node, the first four labelled, and a plain edge out of it.
  const eightLoops = Array.from({ length: 8 }, (_, index) =>
    edge(`e${index}`, 'n0', 'n0', index % 2 === 0 ? [`l${index}`] : [])
  );

  return [
    {
      name: 'synthetic__self-loops-north-sequenced',
      graph: root(
        [loopNode('n0', { [north]: 'NORTH', [ordering]: 'SEQUENCED' }), plainNode('n1')],
        [
          edge('e0', 'n0', 'n0', ['a']),
          edge('e1', 'n0', 'n0', ['bb']),
          edge('e2', 'n0', 'n0', ['ccc']),
          edge('e3', 'n0', 'n1'),
        ]
      ),
    },
    {
      name: 'synthetic__self-loops-north-south-sequenced',
      graph: root(
        [loopNode('n0', { [north]: 'NORTH_SOUTH', [ordering]: 'SEQUENCED' }), plainNode('n1')],
        [
          edge('e0', 'n0', 'n0', ['p']),
          edge('e1', 'n0', 'n0', ['qq']),
          edge('e2', 'n0', 'n0', ['rrr']),
          edge('e3', 'n0', 'n0', ['ssss']),
          edge('e4', 'n0', 'n1'),
        ]
      ),
    },
    {
      name: 'synthetic__self-loops-north-reverse-stacked',
      graph: root(
        [loopNode('n0', { [north]: 'NORTH', [ordering]: 'REVERSE_STACKED' }), plainNode('n1')],
        [
          edge('e0', 'n0', 'n0', ['top', 'bottom']),
          edge('e1', 'n0', 'n0'),
          edge('e2', 'n0', 'n0', ['mid']),
          edge('e3', 'n0', 'n1'),
        ]
      ),
    },
    {
      name: 'synthetic__self-loops-equally-sides',
      graph: root(
        [loopNode('n0', { [north]: 'EQUALLY' }), plainNode('n1')],
        [
          edge('e0', 'n0', 'n0', ['north']),
          edge('e1', 'n0', 'n0', ['south']),
          edge('e2', 'n0', 'n0', ['east']),
          edge('e3', 'n0', 'n0', ['west']),
          edge('e4', 'n0', 'n1'),
        ]
      ),
    },
    {
      name: 'synthetic__self-loops-equally-corners',
      graph: root(
        [loopNode('n0', { [north]: 'EQUALLY' }), plainNode('n1')],
        [...eightLoops, edge('e8', 'n0', 'n1')]
      ),
    },
    {
      name: 'synthetic__self-loops-fixed-side-opposing',
      graph: root(
        [
          loopNode(
            'n0',
            { 'elk.portConstraints': 'FIXED_SIDE' },
            {
              ports: [
                port('p0', 'EAST'),
                port('p1', 'WEST'),
                port('p2', 'NORTH'),
                port('p3', 'SOUTH'),
              ],
            }
          ),
        ],
        [edge('e0', 'p0', 'p1', ['east west']), edge('e1', 'p2', 'p3', ['north south'])]
      ),
    },
  ];
}
