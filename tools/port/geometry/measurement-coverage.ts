/** Coverage diagnostics for Mermaid's internal entity placeholders. */
import { FALLBACK_FILES, LEXICA_FILES, MONO_FILES, faceCoverage } from '../fonts/bundled.ts';

export type FixtureFont = 'lexica' | 'mono';

let fallbackCoverage: Set<number> | undefined;

/**
 * A conservative scan of possible measured placeholders, not a parser. Mermaid encodes #name; and
 * #123; before parsing; source that already contains the sentinel can also reach measurement. The
 * scan may include syntax that a diagram never measures. It is a warning, never an exclusion.
 */
export function placeholderCoverageWarnings(source: string, font: FixtureFont): string[] {
  const text = source.replace(/#(\w+);/g, (_, name: string) =>
    /^\d+$/.test(name) ? `ﬂ°°${name}¶ß` : `ﬂ°${name}¶ß`
  );
  const placeholders = text.match(/ﬂ°°?\w+¶ß/g) ?? [];
  if (placeholders.length === 0) return [];
  fallbackCoverage ??= new Set(FALLBACK_FILES.flatMap((file) => [...faceCoverage(file)]));
  const faces = font === 'mono' ? MONO_FILES : LEXICA_FILES;
  const warnings: string[] = [];
  for (const file of faces) {
    const covered = faceCoverage(file);
    const missing = new Set<string>();
    for (const character of placeholders.join('')) {
      const code = character.codePointAt(0)!;
      if (!covered.has(code) && !fallbackCoverage.has(code)) {
        missing.add(`U+${code.toString(16).toUpperCase().padStart(4, '0')} ${character}`);
      }
    }
    if (missing.size) {
      warnings.push(
        `internal entity placeholder may be measured with characters not covered by ${file} or registered fallbacks (${[...missing].join(', ')}); Chromium may use a system font`
      );
    }
  }
  return warnings;
}
