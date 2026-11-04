/**
 * The adaptive attachment policy applied to the reference's measured ELK input (decision 0019).
 * This runs in Chromium before elkjs, leaving measurement, layout, clipping and SVG writing to the
 * reference. Compatibility runs leave the hook unset. Kept as page source like the geometry probes.
 */
export const ADAPTIVE_PORTS = `
window.__mermaidElkConfigure = function centerPorts(graph) {
  const paths = new Map();
  const heights = new Map();
  const widths = new Map();
  function index(node, ancestors) {
    const path = [...ancestors, node];
    paths.set(node.id, path);
    for (const child of node.children ?? []) index(child, path);
  }
  index(graph, []);
  function collect(node) {
    for (const edge of node.edges ?? []) {
      const source = paths.get(edge.sources?.[0] ?? edge.source);
      const target = paths.get(edge.targets?.[0] ?? edge.target);
      if (!source || !target) continue;
      let common = 0;
      while (common < source.length && common < target.length && source[common] === target[common]) {
        common++;
      }
      const containerDepth = source === target ? source.length - 2 : common - 1;
      if (containerDepth < 0) continue;
      let height = 0;
      let width = 0;
      for (const label of edge.labels ?? []) {
        if (Number.isFinite(label.height)) height = Math.max(height, label.height);
        if (Number.isFinite(label.width)) width = Math.max(width, label.width);
      }
      if (height > 0) {
        for (const endpoint of [source, target]) {
          for (let depth = containerDepth; depth < endpoint.length - 1; depth++) {
            const container = endpoint[depth];
            heights.set(container, Math.max(heights.get(container) ?? 0, height));
            widths.set(container, Math.max(widths.get(container) ?? 0, width));
          }
        }
      }
    }
    for (const child of node.children ?? []) collect(child);
  }
  collect(graph);
  function apply(node, inheritedDirection = 'DOWN') {
    const direction = node !== graph && node.layoutOptions?.['elk.hierarchyHandling'] === 'INCLUDE_CHILDREN'
      ? inheritedDirection
      : node.layoutOptions?.['elk.direction'] ?? inheritedDirection;
    const height = heights.get(node);
    if (height > 0) {
      node.layoutOptions ??= {};
      let spacing = Math.max(20, 2 * height + 2);
      if (direction === 'DOWN' || direction === 'UP') {
        spacing = Math.max(spacing, (widths.get(node) ?? 0) / 2 + 2);
      }
      node.layoutOptions['elk.spacing.portPort'] = spacing;
      for (const child of node.children ?? []) {
        child.layoutOptions ??= {};
        child.layoutOptions['elk.portAlignment.default'] = 'CENTER';
      }
    }
    for (const child of node.children ?? []) apply(child, direction);
  }
  apply(graph);
};
`;
