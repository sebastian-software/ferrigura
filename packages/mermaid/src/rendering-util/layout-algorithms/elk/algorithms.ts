/**
 * ELK algorithms selectable as `layout: <name>`, in addition to the plain `elk`
 * name which maps to `elk.layered`.
 */
export const ELK_ALGORITHMS = [
  'elk.stress',
  'elk.force',
  'elk.mrtree',
  'elk.sporeOverlap',
  'elk.box',
  'elk.rectpacking',
] as const;
