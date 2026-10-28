/**
 * The bundled faces of ferrigura-text, read from their files: the code points each covers, and the
 * faces a label's text is set in. The geometry harness uses them for its exclusions and for the
 * `unicode-range` of the reference page, so both sides draw a character in the same face.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { readCmap } from './cmap.ts';

/** The directory of the bundled faces, `crates/ferrigura-text/fonts`. */
export const BUNDLED_FONT_DIR = fileURLToPath(
  new URL('../../../crates/ferrigura-text/fonts/', import.meta.url)
);

/** The four Lexica Ultralegible styles, the text face of labels. */
export const LEXICA_FILES = [
  'LexicaUltralegible-Regular.ttf',
  'LexicaUltralegible-Italic.ttf',
  'LexicaUltralegible-Bold.ttf',
  'LexicaUltralegible-BoldItalic.ttf',
] as const;

/** The four Atkinson Hyperlegible Mono styles, the face of code. */
export const MONO_FILES = [
  'AtkinsonHyperlegibleMono-Regular.ttf',
  'AtkinsonHyperlegibleMono-Italic.ttf',
  'AtkinsonHyperlegibleMono-Bold.ttf',
  'AtkinsonHyperlegibleMono-BoldItalic.ttf',
] as const;

/** The fallback faces of a text, in the order a browser walks them (`fallback_faces`). */
export const FALLBACK_FILES = [
  'NotoSansMath-Regular-Arrows.ttf',
  'NotoSansSymbols2-Regular-TextSymbols.ttf',
  'TwemojiMozilla-COLRv0-Emoji.ttf',
] as const;

const coverageByFile = new Map<string, Set<number>>();

/** The code points a bundled face maps to a glyph, read from its `cmap` once. */
export function faceCoverage(file: string): Set<number> {
  let covered = coverageByFile.get(file);
  if (!covered) {
    covered = readCmap(new Uint8Array(readFileSync(join(BUNDLED_FONT_DIR, file))));
    coverageByFile.set(file, covered);
  }
  return covered;
}

/**
 * The code points a label can draw: the four Lexica styles and the three fallback faces, as
 * `ferrigura-text` sets a label's text. Atkinson Mono is the face of code and is not a fallback of a
 * label, so a character only the mono face covers is not counted here.
 */
export function labelCoverage(): Set<number> {
  const covered = new Set<number>();
  for (const file of [...LEXICA_FILES, ...FALLBACK_FILES]) {
    for (const code of faceCoverage(file)) {
      covered.add(code);
    }
  }
  return covered;
}

/**
 * The code points that a face the reference's page declares covers, as `unicode-range` ranges in
 * CSS notation (`U+2190-21FF, U+2665`), so that the page's family takes exactly these characters
 * from the face and no other.
 */
export function unicodeRange(codes: Iterable<number>): string {
  const sorted = [...new Set(codes)].sort((a, b) => a - b);
  const ranges: string[] = [];
  let first = -1;
  let last = -1;
  const flush = () => {
    if (first < 0) {
      return;
    }
    const hex = (code: number) => `U+${code.toString(16).toUpperCase()}`;
    ranges.push(first === last ? hex(first) : `${hex(first)}-${last.toString(16).toUpperCase()}`);
  };
  for (const code of sorted) {
    if (code === last + 1) {
      last = code;
    } else {
      flush();
      first = code;
      last = code;
    }
  }
  flush();
  return ranges.join(', ');
}
