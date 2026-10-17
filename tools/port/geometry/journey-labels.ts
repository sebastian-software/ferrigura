/**
 * The labels in the geometry harness (`diagram-geometry.ts`, label mode: the journey and the
 * eventmodeling).
 *
 * The reference draws a journey label of the default `textPlacement` (`fo`) as HTML in a
 * `foreignObject`, inside a `switch` that also holds an SVG `text` fallback which Chromium does not
 * draw. The eventmodeling draws its box text as HTML in a `foreignObject` of the box. ferrigura
 * draws every such label as SVG text, in the lines the browser breaks the HTML into (decision 0004).
 * The element structure cannot pair the two, so in label mode the harness leaves each label out of
 * the element comparison and compares what the browser lays out instead: each line of the label,
 * its text, and its box. A label is a `switch` on ferrigura's side and a `foreignObject` on the
 * reference's, and both are keyed `label[n]` under their parent.
 *
 * A line is found from the characters: a `Range` over each character of the label's text gives its
 * box, the characters with the same top form a line, and the box of a line is the union of the
 * boxes of its characters that are not blanks. The reference's label is the `div.label` of the
 * `foreignObject` (or the `foreignObject` itself, the eventmodeling's box text, which has no
 * `.label`); ferrigura's is the `text` of the `switch`. The boxes are in viewBox units, as the
 * generic harness's boxes are.
 *
 * Everything else in a journey or an eventmodeling is compared by the generic rule
 * (`diagram-compare.ts`).
 */
import { PIXEL_TOLERANCE } from './common.js';

/** One line of a label as the browser lays it out. */
export interface LabelLine {
  /** The text of the line, without the blanks at its ends. */
  text: string;
  /** The union of the boxes of the line's characters, in viewBox units; null for a line of blanks. */
  box: { x: number; y: number; width: number; height: number } | null;
}

/** A label: the structural key of its `switch`, and its lines in order. */
export interface LabelGeometry {
  key: string;
  lines: LabelLine[];
}

/**
 * The settings the journey fixtures get on top of the forced ones (`FORCED_SETTINGS`). The
 * reference sets the title in `titleFontFamily`, which the default names as system fonts
 * (`trebuchet ms`), and the `tspan` placement in `taskFontFamily` (`Open Sans`). Both sides use the
 * bundled face instead, which ferrigura always does. The default `fo` placement does not read the
 * task font, so the pin changes only the fixtures that choose another placement.
 */
export const JOURNEY_SETTINGS: Record<string, unknown> = {
  journey: {
    titleFontFamily: '"Lexica Ultralegible"',
    taskFontFamily: '"Lexica Ultralegible"',
  },
};

/**
 * The page-side measurement of a label's lines, as source text for the page: it defines
 * `labelLines(svg, element)`, using the page's `mapPoint`. It is included by the journey's measure.
 */
export const LABEL_LINES = `
const labelLines = (svg, element) => {
  const root =
    element.tagName === 'foreignObject'
      ? element
      : element.querySelector('foreignObject .label') || element.querySelector('text');
  if (!root) return [];
  const inverse = svg.getScreenCTM().inverse();
  const lines = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    for (let index = 0; index < node.data.length; index++) {
      const range = document.createRange();
      range.setStart(node, index);
      range.setEnd(node, index + 1);
      const rect = range.getBoundingClientRect();
      // A blank the browser collapses has no width (its box has a height, but no width): it is not
      // part of the drawn text, so a run of blanks reads as one blank, as SVG and HTML both draw it.
      if (rect.width === 0) continue;
      // A line feed of a data block is a blank in the layout: both sides read it as one, since the
      // reference keeps the feed in its DOM text and ferrigura writes the blank it draws.
      const character = /[\\t\\n\\r\\f]/.test(node.data[index]) ? ' ' : node.data[index];
      let line = lines[lines.length - 1];
      if (!line || Math.abs(line.top - rect.top) > 0.5) {
        line = { top: rect.top, text: '', rects: [] };
        lines.push(line);
      }
      line.text += character;
      if (character.trim() !== '') line.rects.push(rect);
    }
  }
  return lines.map((line) => {
    const text = line.text.trim();
    if (line.rects.length === 0) return { text: text, box: null };
    const left = Math.min(...line.rects.map((rect) => rect.left));
    const right = Math.max(...line.rects.map((rect) => rect.right));
    const top = Math.min(...line.rects.map((rect) => rect.top));
    const bottom = Math.max(...line.rects.map((rect) => rect.bottom));
    const corner = mapPoint(svg, inverse, left, top);
    const far = mapPoint(svg, inverse, right, bottom);
    return {
      text: text,
      box: {
        x: Math.min(corner.x, far.x),
        y: Math.min(corner.y, far.y),
        width: Math.abs(far.x - corner.x),
        height: Math.abs(far.y - corner.y),
      },
    };
  });
};
`;

/** The worst difference as a fraction of the tolerance, and the problems, of the labels. */
export interface LabelComparison {
  worst: number | null;
  problems: string[];
}

/**
 * Compares the labels of the reference with the other side's, by their keys. Each key must be on
 * both sides, with the same number of lines; each line must have the same text, and the same box
 * within the tolerance. The worst difference is the largest one as a fraction of the tolerance.
 */
export function compareLabels(
  reference: LabelGeometry[],
  other: LabelGeometry[],
  otherName: string
): LabelComparison {
  let worst: number | null = null;
  const problems: string[] = [];
  const number = (actual: number, expected: number, where: string): void => {
    if (!Number.isFinite(actual) || !Number.isFinite(expected)) {
      problems.push(`${where}: not a number (${otherName} ${actual}, reference ${expected})`);
      return;
    }
    const difference = Math.abs(actual - expected);
    worst = Math.max(worst ?? 0, difference / PIXEL_TOLERANCE);
    if (difference > PIXEL_TOLERANCE) {
      problems.push(
        `${where}: ${difference.toFixed(4)} px apart (${otherName} ${actual.toFixed(3)}, reference ${expected.toFixed(3)})`
      );
    }
  };
  const otherByKey = new Map(other.map((label) => [label.key, label]));
  const referenceKeys = new Set(reference.map((label) => label.key));
  for (const expected of reference) {
    const actual = otherByKey.get(expected.key);
    if (!actual) {
      problems.push(`${expected.key}: a label only in the reference, missing in ${otherName}`);
      continue;
    }
    if (actual.lines.length !== expected.lines.length) {
      problems.push(
        `${expected.key} lines: ${otherName} ${actual.lines.length} (${JSON.stringify(actual.lines.map((line) => line.text))}), reference ${expected.lines.length} (${JSON.stringify(expected.lines.map((line) => line.text))})`
      );
      continue;
    }
    expected.lines.forEach((line, index) => {
      const where = `${expected.key} line ${index}`;
      const found = actual.lines[index];
      if (found.text !== line.text) {
        problems.push(
          `${where} text: ${otherName} ${JSON.stringify(found.text)}, reference ${JSON.stringify(line.text)}`
        );
      }
      if (line.box === null || found.box === null) {
        if (line.box !== found.box) problems.push(`${where} box: missing on one side`);
        return;
      }
      number(found.box.x, line.box.x, `${where} x`);
      number(found.box.y, line.box.y, `${where} y`);
      number(found.box.width, line.box.width, `${where} width`);
      number(found.box.height, line.box.height, `${where} height`);
    });
  }
  for (const actual of other) {
    if (!referenceKeys.has(actual.key)) {
      problems.push(`${actual.key}: a label only in ${otherName}, missing in the reference`);
    }
  }
  return { worst, problems };
}
