/**
 * Configuration options and theme variables that upstream Mermaid 12.2.0 defines but that no code
 * reads, its own included. ferrigura leaves them out (docs-port/ORACLE-PLAN.md, decision 1), as
 * this repository's fork did (FORK.md, Configuration), so the configuration and theme runs leave
 * them out on both sides: one rule instead of an exception per input.
 */
export const INERT_OPTIONS: readonly string[] = [
  'legacyMathML',
  'er.minEntityHeight',
  'class.dividerMargin',
  'state.sizeUnit',
  'state.titleShift',
  'state.forkWidth',
  'state.forkHeight',
  'state.miniPadding',
  'state.fontSizeFactor',
  'state.edgeLengthFactor',
  'state.compositTitleSize',
  'state.dividerMargin',
  'themeVariables.attributeBackgroundColorOdd',
  'themeVariables.attributeBackgroundColorEven',
];

/** The theme variables among the inert options, as the theme run names them. */
export const INERT_THEME_VARIABLES: readonly string[] = INERT_OPTIONS.filter((path) =>
  path.startsWith('themeVariables.')
).map((path) => path.slice('themeVariables.'.length));

/** The last segments of the inert options: the names the key set of the directives lists. */
export const INERT_KEYS: ReadonlySet<string> = new Set(
  INERT_OPTIONS.map((path) => path.slice(path.lastIndexOf('.') + 1))
);

/** A copy of `config` without the inert options; other values are shared, not copied. */
export function withoutInertOptions(config: unknown): unknown {
  if (typeof config !== 'object' || config === null) return config;
  const copy: Record<string, unknown> = { ...(config as Record<string, unknown>) };
  for (const path of INERT_OPTIONS) {
    const dot = path.indexOf('.');
    if (dot === -1) {
      delete copy[path];
      continue;
    }
    const section = path.slice(0, dot);
    const value = copy[section];
    if (typeof value === 'object' && value !== null) {
      const sectionCopy = { ...(value as Record<string, unknown>) };
      delete sectionCopy[path.slice(dot + 1)];
      copy[section] = sectionCopy;
    }
  }
  return copy;
}
