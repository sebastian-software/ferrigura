/**
 * Comparison of two measured SVGs for the generic geometry harness (`diagram-geometry.ts`): the
 * reference and the other side, which is ferrigura or a second reference render. Pure functions, so
 * that the rules can be checked without a browser.
 *
 * Elements are paired by their structural key only: the chain of `tag.class1.class2[n]` from the
 * root, where `n` counts the earlier siblings with the same tag and classes. Ids are recorded but
 * not part of the key, since they may carry the diagram id or a counter. A key on one side only is
 * a problem of its own, listed with its path.
 */
import { PIXEL_TOLERANCE } from './common.js';

export interface Point {
  x: number;
  y: number;
}

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** What the page measures of one element below the root. */
export interface ElementGeometry {
  /** The structural key, for example `svg/g.nodes[0]/g.node.default[2]`. */
  key: string;
  tag: string;
  /** Recorded, not compared. */
  id: string | null;
  /** A `g`: its transform attribute, recorded and not compared (its formatting may differ). */
  transform: string | null;
  /** A `g`: the origin of its user space in viewBox units, which is its translation. */
  origin: Point | null;
  /** The box of a rect, circle, ellipse, line, polygon, polyline, image or text, in viewBox units. */
  box: Box | null;
  /** A text: its text content, compared exactly. */
  text: string | null;
  /** A path: its length and its samples along it, in viewBox units. */
  length: number | null;
  samples: Point[] | null;
  /**
   * Per sample, the two ends when the sample falls on a jump between two sub-paths (a move), where
   * the point the browser returns depends on the last bits of the lengths before it; else null.
   */
  jumps: (Point[] | null)[] | null;
}

/** What the page measures of one SVG. */
export interface DiagramGeometry {
  id: string;
  /** The viewBox as the SVG wrote it. */
  viewBox: number[];
  style: string | null;
  roledescription: string | null;
  /** Every element below the root, in document order, skipping `style`, `defs`, `marker`, `title`, `desc`. */
  elements: ElementGeometry[];
}

export interface Comparison {
  /** The largest difference as a fraction of the tolerance, or null when nothing was compared. */
  worst: number | null;
  problems: string[];
}

/** Whether two points are within the tolerance on both axes. */
function near(a: Point, b: Point): boolean {
  return Math.abs(a.x - b.x) <= PIXEL_TOLERANCE && Math.abs(a.y - b.y) <= PIXEL_TOLERANCE;
}

/** Collects the numeric and exact differences and the worst one, as a fraction of tolerance. */
class Checker {
  worst: number | null = null;
  readonly problems: string[] = [];

  number(actual: number | undefined, expected: number | undefined, where: string): void {
    // The same non-finite number on both sides agrees: a NaN from a configuration the reference
    // computes with (a `bitsPerRow` that is not a number) is written as NaN by both.
    if (typeof actual === 'number' && typeof expected === 'number' && Object.is(actual, expected)) {
      return;
    }
    if (
      actual === undefined ||
      expected === undefined ||
      !Number.isFinite(actual) ||
      !Number.isFinite(expected)
    ) {
      this.problems.push(`${where}: not a number (ferrigura ${actual}, reference ${expected})`);
      return;
    }
    const difference = Math.abs(actual - expected);
    this.worst = Math.max(this.worst ?? 0, difference / PIXEL_TOLERANCE);
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

  exact(actual: string | null, expected: string | null, where: string): void {
    if (actual !== expected) {
      this.problems.push(
        `${where}: ferrigura ${JSON.stringify(actual)}, reference ${JSON.stringify(expected)}`
      );
    }
  }
}

/** The `max-width` of an SVG's style in px, or null. */
function maxWidth(style: string | null): number | null {
  const match = style ? /max-width:\s*([\d+.Ee-]+)px/.exec(style) : null;
  return match ? Number(match[1]) : null;
}

/** Compares the element that both sides have under one key. */
function compareElement(
  checker: Checker,
  expected: ElementGeometry,
  actual: ElementGeometry
): void {
  const key = expected.key;
  if (expected.tag === 'g') checker.point(actual.origin, expected.origin, `${key} origin`);
  checker.box(actual.box, expected.box, `${key} box`);
  if (expected.text !== null || actual.text !== null) {
    checker.exact(actual.text, expected.text, `${key} text`);
  }
  if (expected.length !== null || actual.length !== null) {
    checker.number(actual.length ?? undefined, expected.length ?? undefined, `${key} length`);
  }
  if (expected.samples !== null || actual.samples !== null) {
    if (expected.samples === null || actual.samples === null) {
      checker.problems.push(`${key} samples: missing on one side`);
    } else {
      expected.samples.forEach((point, index) => {
        const sample = actual.samples?.[index] ?? null;
        // On a jump, a sample matches when any end on one side meets any end on the other.
        const expectedEnds = [point, ...(expected.jumps?.[index] ?? [])];
        const actualEnds = sample === null ? [] : [sample, ...(actual.jumps?.[index] ?? [])];
        const onJump = expectedEnds.length > 1 || actualEnds.length > 1;
        if (onJump && actualEnds.some((end) => expectedEnds.some((other) => near(end, other)))) {
          return;
        }
        checker.point(sample, point, `${key} sample ${index}`);
      });
    }
  }
}

/**
 * Compares the reference with the other side. `otherName` names the other side in the problems:
 * `ferrigura`, or `the second reference render` for the self check.
 */
export function compareDiagrams(
  reference: DiagramGeometry,
  other: DiagramGeometry,
  otherName: string
): Comparison {
  const checker = new Checker();

  // Root: the viewBox and the max-width (the svg itself is shown at scale 1 by the page).
  if (reference.viewBox.length !== other.viewBox.length) {
    checker.problems.push(
      `viewBox: ${other.viewBox.length} numbers, reference ${reference.viewBox.length}`
    );
  } else {
    reference.viewBox.forEach((value, index) =>
      checker.number(other.viewBox[index], value, `viewBox[${index}]`)
    );
  }
  const referenceMax = maxWidth(reference.style);
  const otherMax = maxWidth(other.style);
  if (referenceMax !== null || otherMax !== null) {
    if (referenceMax === null || otherMax === null) {
      checker.problems.push(
        `max-width: ${otherName} ${otherMax ?? 'none'}, reference ${referenceMax ?? 'none'}`
      );
    } else {
      checker.number(otherMax, referenceMax, 'max-width');
    }
  }

  // Elements, paired by key, in the reference's document order.
  const otherByKey = new Map(other.elements.map((element) => [element.key, element]));
  const referenceKeys = new Set(reference.elements.map((element) => element.key));
  for (const expected of reference.elements) {
    const actual = otherByKey.get(expected.key);
    if (!actual) {
      checker.problems.push(
        `${expected.key}: only in the reference (${expected.tag}), missing in ${otherName}`
      );
      continue;
    }
    compareElement(checker, expected, actual);
  }
  for (const actual of other.elements) {
    if (!referenceKeys.has(actual.key)) {
      checker.problems.push(
        `${actual.key}: only in ${otherName} (${actual.tag}), missing in the reference`
      );
    }
  }

  return { worst: checker.worst, problems: checker.problems };
}
