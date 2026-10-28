/**
 * Seeded generator of `venn-beta` texts for the venn corpus (see capture-corpus.ts).
 *
 * Pure: it touches neither the browser nor the filesystem. The same seed and count always give the
 * same texts, so the corpus can be regenerated exactly. Every text declares its sets before its
 * unions, as the diagram requires (a union names only known sets). The sets are declared bare or
 * quoted, with numeric, word, alphanumeric or mixed ids; a set may be declared twice, and a union
 * may repeat a set or be declared twice with another size. Sizes are the parser's numbers (no
 * exponent form, no negative sizes): zero, small and large values, the default, and overlaps that
 * are none, all, a part, or more than the smaller set.
 */

/** Seed of the generated set. Changing it changes every generated case. */
export const GENERATOR_SEED = 20261018;

/** Number of generated set systems. */
export const GENERATED_COUNT = 400;

/** The id schemes. The cases cycle through them, so every scheme appears. */
export const ID_SCHEMES = ['plain', 'numeric', 'words', 'alphanumeric', 'mixed'] as const;
export type IdScheme = (typeof ID_SCHEMES)[number];

/** A generated set system: the text it was written as, and how it was made. */
export interface GeneratedSetSystem {
  index: number;
  scheme: IdScheme;
  setCount: number;
  text: string;
}

/** The seeded generator of the harness (mulberry32), so the corpus does not depend on Math.random. */
function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const PLAIN_IDS = [
  'A',
  'B',
  'C',
  'D',
  'E',
  'F',
  'Frontend',
  'Backend',
  'DevOps',
  'node-2',
  'set_x',
];
const NUMERIC_IDS = ['1', '2', '3', '10', '11', '20', '100', '9'];
const WORD_IDS = ['JavaScript', 'Python', 'TypeScript', 'Front end', 'Go-lang', 'C#', 'Rust'];
const ALPHANUMERIC_IDS = ['a1', 'a2', 'a10', 'x2', 'x10', 'x1', 'b3', 'b30'];
const SIZES = [0, 0.5, 1, 2, 3.25, 4, 5, 6.5, 8, 10, 12, 15, 20, 25, 30, 42, 50, 100];

/** Identifiers of the scheme, shuffled, with the ones that need quotes. */
function pickIds(next: () => number, scheme: IdScheme, count: number): string[] {
  const pool: string[] = {
    plain: PLAIN_IDS,
    numeric: NUMERIC_IDS,
    words: WORD_IDS,
    alphanumeric: ALPHANUMERIC_IDS,
    mixed: [...NUMERIC_IDS.slice(0, 4), ...WORD_IDS.slice(0, 3), ...PLAIN_IDS.slice(0, 3)],
  }[scheme];
  const shuffled = [...pool];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(next() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled.slice(0, count);
}

/** The text of an id: quoted unless it is a plain identifier. */
function written(id: string): string {
  return /^[A-Z_a-z][\w-]*$/.test(id) ? id : `"${id}"`;
}

/** A size as the parser reads it: a plain decimal, never an exponent form. */
function sizeText(value: number): string {
  return String(Number(value.toFixed(2)));
}

/** Generates `count` set systems. Each one has two to six sets, cycling through the id schemes. */
export function generateSetSystems(count: number, seed: number): GeneratedSetSystem[] {
  const next = generator(seed);
  const systems: GeneratedSetSystem[] = [];
  for (let index = 0; index < count; index++) {
    const scheme = ID_SCHEMES[index % ID_SCHEMES.length];
    const setCount = 2 + Math.floor(next() * 5);
    const ids = pickIds(next, scheme, setCount);
    const lines: string[] = ['venn-beta'];
    const sizes = new Map<string, number>();

    for (const id of ids) {
      if (next() < 0.6) {
        const size = SIZES[Math.floor(next() * SIZES.length)];
        sizes.set(id, size);
        lines.push(`  set ${written(id)}:${sizeText(size)}`);
      } else {
        sizes.set(id, 10);
        lines.push(`  set ${written(id)}`);
      }
      if (next() < 0.1) {
        // A set declared twice: the parser keeps both entries.
        lines.push(`  set ${written(id)}`);
      }
    }

    const unionCount = Math.floor(next() * 9);
    for (let u = 0; u < unionCount; u++) {
      const size = 2 + Math.floor(next() * (setCount - 1));
      const members: string[] = [];
      const order = [...ids];
      for (let i = order.length - 1; i > 0; i--) {
        const j = Math.floor(next() * (i + 1));
        [order[i], order[j]] = [order[j], order[i]];
      }
      members.push(...order.slice(0, size));
      if (next() < 0.03) {
        // A union that names one set twice.
        members.push(members[0]);
      }
      const smallest = Math.min(...members.map((member) => sizes.get(member) ?? 10));
      const kind = next();
      let overlap: number | null;
      if (kind < 0.35) {
        overlap = null; // the default: 10 / n²
      } else if (kind < 0.45) {
        overlap = 0;
      } else if (kind < 0.6) {
        overlap = smallest; // complete overlap of the smaller set
      } else if (kind < 0.7) {
        overlap = smallest * 1.5; // more than the smaller set
      } else {
        overlap = Math.round(next() * smallest * 100) / 100;
      }
      const list = members.map(written).join(', ');
      lines.push(overlap === null ? `  union ${list}` : `  union ${list}:${sizeText(overlap)}`);
      if (next() < 0.08) {
        lines.push(`  union ${list}:${sizeText(smallest / 3)}`);
      }
    }

    systems.push({ index, scheme, setCount, text: lines.join('\n') });
  }
  return systems;
}
