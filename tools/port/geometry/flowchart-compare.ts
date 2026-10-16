/**
 * Comparison of the two sides of the flowchart geometry harness: the measured SVG geometry of the
 * reference and of ferrigura, and the ELK input of the reference and of ferrigura. Pure functions,
 * so that the rules can be checked without a browser (`flowchart-geometry.ts` holds the page and
 * the run).
 */

export const PIXEL_TOLERANCE = 0.05;

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface NodeGeometry {
  id: string;
  transform: string | null;
  centre: Point | null;
  shape: { source: string; tag: string | null; box: Box | null };
  label: Box | null;
}

export interface EdgeGeometry {
  id: string;
  d: string | null;
  length: number;
  samples: Point[];
  dataPoints: Point[] | 'invalid' | null;
  markerStart: string | null;
  markerEnd: string | null;
}

export interface EdgeLabelGeometry {
  dataId: string | null;
  box: Box | null;
  background: Box | null;
}

export interface ClusterGeometry {
  id: string;
  rect: Box | null;
  label: Box | null;
}

/** What the page measures of one SVG. */
export interface SvgGeometry {
  id: string;
  viewBox: number[];
  width: string | null;
  height: string | null;
  style: string | null;
  roledescription: string | null;
  looks: string[];
  foreignObjects: number;
  nodes: NodeGeometry[];
  edges: EdgeGeometry[];
  edgeLabels: EdgeLabelGeometry[];
  clusters: ClusterGeometry[];
  counts: Record<string, number>;
}

/** Collects the differences between the two sides and the worst one, as a fraction of tolerance. */
class Checker {
  worst: number | null = null;
  readonly problems: string[] = [];

  number(actual: number, expected: number, where: string): void {
    if (!Number.isFinite(actual) || !Number.isFinite(expected)) {
      this.problems.push(`${where}: not a number (ferrigura ${actual}, reference ${expected})`);
      return;
    }
    const difference = Math.abs(actual - expected);
    const ratio = difference / PIXEL_TOLERANCE;
    this.worst = Math.max(this.worst ?? 0, ratio);
    if (difference > PIXEL_TOLERANCE) {
      this.problems.push(
        `${where}: ${difference.toFixed(4)} px apart (ferrigura ${actual.toFixed(3)}, reference ${expected.toFixed(3)})`
      );
    }
  }

  point(actual: Point | null, expected: Point | null, where: string): void {
    if (actual === null || expected === null) {
      if (actual !== expected) this.problems.push(`${where}: missing on one side`);
      return;
    }
    this.number(actual.x, expected.x, `${where} x`);
    this.number(actual.y, expected.y, `${where} y`);
  }

  box(actual: Box | null, expected: Box | null, where: string): void {
    if (actual === null || expected === null) {
      if (actual !== expected) this.problems.push(`${where}: missing on one side`);
      return;
    }
    this.number(actual.x, expected.x, `${where} x`);
    this.number(actual.y, expected.y, `${where} y`);
    this.number(actual.width, expected.width, `${where} width`);
    this.number(actual.height, expected.height, `${where} height`);
  }

  /** A size that ELK may leave out (it computes group sizes): absent on both sides is equal. */
  optionalNumber(
    actual: number | null | undefined,
    expected: number | null | undefined,
    where: string
  ): void {
    // ferrigura's JSON writes an absent size as null, the reference leaves the key out.
    if (actual == null && expected == null) return;
    if (actual == null || expected == null) {
      this.problems.push(
        `${where}: ferrigura ${actual ?? 'absent'}, reference ${expected ?? 'absent'}`
      );
      return;
    }
    this.number(actual, expected, where);
  }

  /** Layout options must match exactly; the problems name the keys that differ. */
  options(
    actual: Record<string, unknown> | undefined,
    expected: Record<string, unknown> | undefined,
    where: string
  ): void {
    const left = actual ?? {};
    const right = expected ?? {};
    const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
    for (const key of keys) {
      if (canonical(left[key]) !== canonical(right[key])) {
        const show = (value: unknown) => (value === undefined ? 'absent' : JSON.stringify(value));
        this.problems.push(
          `${where} ${key}: ferrigura ${show(left[key])}, reference ${show(right[key])}`
        );
      }
    }
  }

  exact(actual: unknown, expected: unknown, where: string): void {
    if (actual !== expected) {
      this.problems.push(
        `${where}: ferrigura ${JSON.stringify(actual)}, reference ${JSON.stringify(expected)}`
      );
    }
  }

  missing(where: string, side: 'ferrigura' | 'reference'): void {
    this.problems.push(`${where}: missing on ${side}`);
  }
}

/** The part of a key that is the same on both sides: the root id (diagram prefix) stripped. */
function stripPrefix(value: string, prefix: string): string {
  return value.startsWith(prefix) ? value.slice(prefix.length) : value;
}

/** A node's vertex id: the diagram prefix, `flowchart-` and the trailing counter stripped. */
function nodeKey(id: string, rootId: string): string {
  return stripPrefix(id, `${rootId}-flowchart-`).replace(/-\d+$/, '');
}

/** A marker's kind from its url, for example `pointEnd-margin`, without the ids around it. */
function markerKind(url: string | null, rootId: string): string | null {
  if (url === null) return null;
  const match = /url\(#([^)]+)\)/.exec(url);
  const id = match ? match[1] : url;
  return stripPrefix(id, `${rootId}_`).replace(/^flowchart-v2-/, '');
}

/**
 * Compares the two measured SVGs of one fixture. Elements are paired by key (vertex id, edge id,
 * data-id, subgraph id), never by counter or by position; a key on one side only is a problem.
 */
export function compareGeometry(
  reference: SvgGeometry,
  ferrigura: SvgGeometry
): { worst: number | null; problems: string[] } {
  const checker = new Checker();

  // Root: the viewBox and the max-width (the svg itself has width 100%).
  if (reference.viewBox.length !== ferrigura.viewBox.length) {
    checker.problems.push(
      `viewBox: ${ferrigura.viewBox.length} numbers, reference ${reference.viewBox.length}`
    );
  } else {
    reference.viewBox.forEach((value, index) =>
      checker.number(ferrigura.viewBox[index], value, `viewBox[${index}]`)
    );
  }
  const maxWidth = (style: string | null): number | null => {
    const match = style ? /max-width:\s*([\d+.Ee-]+)px/.exec(style) : null;
    return match ? Number(match[1]) : null;
  };
  const referenceMax = maxWidth(reference.style);
  const ferriguraMax = maxWidth(ferrigura.style);
  if (referenceMax !== null || ferriguraMax !== null) {
    if (referenceMax === null || ferriguraMax === null) {
      checker.problems.push(
        `max-width: ferrigura ${ferriguraMax ?? 'none'}, reference ${referenceMax ?? 'none'}`
      );
    } else {
      checker.number(ferriguraMax, referenceMax, 'max-width');
    }
  }

  // Nodes, keyed by vertex id.
  const referenceNodes = new Map(
    reference.nodes.map((node) => [nodeKey(node.id, reference.id), node])
  );
  const ferriguraNodes = new Map(
    ferrigura.nodes.map((node) => [nodeKey(node.id, ferrigura.id), node])
  );
  for (const [key, expected] of referenceNodes) {
    const actual = ferriguraNodes.get(key);
    if (!actual) {
      checker.missing(`node ${key}`, 'ferrigura');
      continue;
    }
    checker.point(actual.centre, expected.centre, `node ${key} centre`);
    checker.exact(actual.shape.source, expected.shape.source, `node ${key} shape source`);
    checker.box(actual.shape.box, expected.shape.box, `node ${key} shape box`);
    checker.box(actual.label, expected.label, `node ${key} label box`);
  }
  for (const key of ferriguraNodes.keys()) {
    if (!referenceNodes.has(key)) checker.missing(`node ${key}`, 'reference');
  }

  // Edges, keyed by edge id.
  const referenceEdges = new Map(
    reference.edges.map((edge) => [stripPrefix(edge.id, `${reference.id}-`), edge])
  );
  const ferriguraEdges = new Map(
    ferrigura.edges.map((edge) => [stripPrefix(edge.id, `${ferrigura.id}-`), edge])
  );
  for (const [key, expected] of referenceEdges) {
    const actual = ferriguraEdges.get(key);
    if (!actual) {
      checker.missing(`edge ${key}`, 'ferrigura');
      continue;
    }
    checker.number(actual.length, expected.length, `edge ${key} length`);
    expected.samples.forEach((point, index) =>
      checker.point(actual.samples[index] ?? null, point, `edge ${key} sample ${index}`)
    );
    const expectedPoints = expected.dataPoints;
    const actualPoints = actual.dataPoints;
    if (Array.isArray(expectedPoints) && Array.isArray(actualPoints)) {
      if (expectedPoints.length !== actualPoints.length) {
        checker.problems.push(
          `edge ${key} data-points: ${actualPoints.length} points, reference ${expectedPoints.length}`
        );
      } else {
        expectedPoints.forEach((point, index) =>
          checker.point(actualPoints[index], point, `edge ${key} data-points ${index}`)
        );
      }
    }
    checker.exact(
      markerKind(actual.markerStart, ferrigura.id),
      markerKind(expected.markerStart, reference.id),
      `edge ${key} marker-start`
    );
    checker.exact(
      markerKind(actual.markerEnd, ferrigura.id),
      markerKind(expected.markerEnd, reference.id),
      `edge ${key} marker-end`
    );
  }
  for (const key of ferriguraEdges.keys()) {
    if (!referenceEdges.has(key)) checker.missing(`edge ${key}`, 'reference');
  }

  // Edge labels, keyed by data-id (the edge id without the diagram prefix).
  const referenceLabels = new Map(
    reference.edgeLabels.map((label) => [
      stripPrefix(label.dataId ?? '', `${reference.id}-`),
      label,
    ])
  );
  const ferriguraLabels = new Map(
    ferrigura.edgeLabels.map((label) => [
      stripPrefix(label.dataId ?? '', `${ferrigura.id}-`),
      label,
    ])
  );
  for (const [key, expected] of referenceLabels) {
    const actual = ferriguraLabels.get(key);
    if (!actual) {
      checker.missing(`edge label ${key}`, 'ferrigura');
      continue;
    }
    checker.box(actual.box, expected.box, `edge label ${key} box`);
    checker.box(actual.background, expected.background, `edge label ${key} background`);
  }
  for (const key of ferriguraLabels.keys()) {
    if (!referenceLabels.has(key)) checker.missing(`edge label ${key}`, 'reference');
  }

  // Clusters, keyed by subgraph id.
  const referenceClusters = new Map(
    reference.clusters.map((cluster) => [stripPrefix(cluster.id, `${reference.id}-`), cluster])
  );
  const ferriguraClusters = new Map(
    ferrigura.clusters.map((cluster) => [stripPrefix(cluster.id, `${ferrigura.id}-`), cluster])
  );
  for (const [key, expected] of referenceClusters) {
    const actual = ferriguraClusters.get(key);
    if (!actual) {
      checker.missing(`cluster ${key}`, 'ferrigura');
      continue;
    }
    checker.box(actual.rect, expected.rect, `cluster ${key} rect`);
    checker.box(actual.label, expected.label, `cluster ${key} label`);
  }
  for (const key of ferriguraClusters.keys()) {
    if (!referenceClusters.has(key)) checker.missing(`cluster ${key}`, 'reference');
  }

  // Element counts per class: exact.
  const classes = new Set([...Object.keys(reference.counts), ...Object.keys(ferrigura.counts)]);
  for (const name of [...classes].sort()) {
    checker.exact(
      ferrigura.counts[name] ?? 0,
      reference.counts[name] ?? 0,
      `count of class ${name}`
    );
  }

  return { worst: checker.worst, problems: checker.problems };
}

/** The ELK input of one render: the nodes and edges in order, their sizes, labels and options. */
export interface ElkElement {
  id?: string;
  width?: number;
  height?: number;
  labels?: { text?: string; width?: number; height?: number }[];
  layoutOptions?: Record<string, unknown>;
  sources?: string[];
  targets?: string[];
  children?: ElkElement[];
  edges?: ElkElement[];
}

/** Every node, and every edge with its container, in the order elkjs reads them. */
function flattenElk(graph: ElkElement): { nodes: ElkElement[]; edges: ElkElement[] } {
  const nodes: ElkElement[] = [];
  const edges: ElkElement[] = [];
  const visit = (container: ElkElement): void => {
    for (const edge of container.edges ?? []) edges.push(edge);
    for (const child of container.children ?? []) {
      nodes.push(child);
      visit(child);
    }
  };
  visit(graph);
  return { nodes, edges };
}

/** A canonical JSON string, so that the key order of an object does not matter. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>).sort(([a], [b]) =>
      a < b ? -1 : 1
    );
    return `{${entries.map(([key, item]) => `${JSON.stringify(key)}:${canonical(item)}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

/** Compares a ferrigura ELK input with the reference's: ids, order, sizes, labels, options. */
export function compareElkInput(
  reference: ElkElement,
  ferrigura: ElkElement
): { status: 'match' | 'differ'; problems: string[] } {
  const checker = new Checker();
  const refFlat = flattenElk(reference);
  const fsFlat = flattenElk(ferrigura);
  const ids = (list: ElkElement[]) => list.map((item) => item.id ?? '');
  if (canonical(ids(fsFlat.nodes)) !== canonical(ids(refFlat.nodes))) {
    checker.problems.push(
      `node ids and order: ferrigura [${ids(fsFlat.nodes).join(', ')}], reference [${ids(refFlat.nodes).join(', ')}]`
    );
  }
  if (canonical(ids(fsFlat.edges)) !== canonical(ids(refFlat.edges))) {
    checker.problems.push(
      `edge ids and order: ferrigura [${ids(fsFlat.edges).join(', ')}], reference [${ids(refFlat.edges).join(', ')}]`
    );
  }
  checker.options(ferrigura.layoutOptions, reference.layoutOptions, 'root layoutOptions');
  // Labels and layout options are compared for nodes and edges; width and height for nodes only.
  const labelsAndOptions = (where: string, actual: ElkElement, expected: ElkElement) => {
    const actualLabels = actual.labels ?? [];
    const expectedLabels = expected.labels ?? [];
    if (actualLabels.length !== expectedLabels.length) {
      checker.problems.push(
        `${where} labels: ${actualLabels.length}, reference ${expectedLabels.length}`
      );
    }
    expectedLabels.forEach((label, index) => {
      const other = actualLabels[index];
      if (!other) return;
      checker.exact(other.text, label.text, `${where} label ${index} text`);
      checker.optionalNumber(other.width, label.width, `${where} label ${index} width`);
      checker.optionalNumber(other.height, label.height, `${where} label ${index} height`);
    });
    checker.options(actual.layoutOptions, expected.layoutOptions, `${where} layoutOptions`);
  };
  refFlat.nodes.forEach((node, index) => {
    const other = fsFlat.nodes[index];
    if (!other) return;
    checker.optionalNumber(other.width, node.width, `node ${node.id} width`);
    checker.optionalNumber(other.height, node.height, `node ${node.id} height`);
    labelsAndOptions(`node ${node.id}`, other, node);
  });
  refFlat.edges.forEach((edge, index) => {
    const other = fsFlat.edges[index];
    if (!other) return;
    checker.exact(canonical(other.sources), canonical(edge.sources), `edge ${edge.id} sources`);
    checker.exact(canonical(other.targets), canonical(edge.targets), `edge ${edge.id} targets`);
    labelsAndOptions(`edge ${edge.id}`, other, edge);
  });
  return { status: checker.problems.length === 0 ? 'match' : 'differ', problems: checker.problems };
}
