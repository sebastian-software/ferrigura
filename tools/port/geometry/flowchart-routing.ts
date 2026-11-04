/**
 * The adaptive attachment policy applied to the reference's measured ELK input (decision 0019).
 * This runs in Chromium before elkjs, leaving measurement, layout, clipping and SVG writing to the
 * reference. Compatibility runs leave the hook unset. Kept as page source like the geometry probes.
 */
export const ADAPTIVE_PORTS = `
window.__mermaidElkConfigure = function centerPorts(graph) {
  if (!graph.children?.length) return;
  let labelHeight = 0;
  for (const edge of graph.edges ?? []) {
    for (const label of edge.labels ?? []) {
      if (Number.isFinite(label.height)) labelHeight = Math.max(labelHeight, label.height);
    }
  }
  if (labelHeight <= 0) {
    for (const child of graph.children) centerPorts(child);
    return;
  }
  graph.layoutOptions ??= {};
  graph.layoutOptions['elk.spacing.portPort'] = Math.max(20, 2 * labelHeight + 2);
  for (const child of graph.children) {
    child.layoutOptions ??= {};
    child.layoutOptions['elk.portAlignment.default'] = 'CENTER';
    centerPorts(child);
  }
};
`;
