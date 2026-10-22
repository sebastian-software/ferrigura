/**
 * Generator of seeded architecture diagrams for the fcose differential corpus
 * (capture-fcose-corpus.ts, at the tag fork-mermaid-12.2.0).
 *
 * The diagrams use the grammar of packages/parser/src/language/architecture/architecture.langium:
 * groups nested up to three levels deep, services with and without icons, junctions, edges between
 * services and junctions with every pair of sides and every arrow form, `{group}` modifiers, and
 * `align row|column` hints. Each diagram carries the layout config that the capture passes to
 * Mermaid under `architecture`. The seeds are 1 to 9, never 0: seed 0 keeps the real Math.random
 * (architectureSeed.ts), and a layout from it does not repeat.
 *
 * One mulberry32 stream (the algorithm of architectureSeed.ts), started from a fixed seed, makes the
 * same seed and count always give the same diagrams. Some diagrams do not render (an align chain
 * that contradicts the edge directions, for instance). The capture skips those; the generator does
 * not try to avoid them, so the corpus also covers the failures' neighbourhood.
 */

/** Seed of the generator's stream. Changing it changes every generated case. */
export const GENERATOR_SEED = 20261017;
/** Number of diagrams generated. The capture renders each one; renders that fail are skipped. */
export const GENERATED_COUNT = 400;

const ICONS = ['database', 'server', 'disk', 'internet'] as const;
const SIDES = ['L', 'R', 'T', 'B'] as const;
type Side = (typeof SIDES)[number];
const OPPOSITE: Record<Side, Side> = { L: 'R', R: 'L', T: 'B', B: 'T' };

/** The layout config of one diagram, as the capture passes it to Mermaid under `architecture`. */
export interface ArchitectureConfig {
  seed: number;
  randomize: boolean;
  nodeSeparation: number;
  numIter: number;
  idealEdgeLengthMultiplier: number;
  edgeElasticity: number;
}

export interface GeneratedArchitecture {
  index: number;
  text: string;
  config: ArchitectureConfig;
  /** What the diagram contains, for the capture's counts. */
  shape: {
    groups: number;
    maxDepth: number;
    services: number;
    junctions: number;
    edges: number;
    alignments: number;
  };
}

/** The mulberry32 generator of architectureSeed.ts, started from `seed`. */
function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

interface Rng {
  /** A uniform integer in [lo, hi]. */
  int(lo: number, hi: number): number;
  /** True with probability p. */
  chance(p: number): boolean;
  pick<T>(items: readonly T[]): T;
}

function rngFrom(seed: number): Rng {
  const next = mulberry32(seed);
  return {
    int: (lo, hi) => lo + Math.floor(next() * (hi - lo + 1)),
    chance: (p) => next() < p,
    pick: (items) => items[Math.floor(next() * items.length)],
  };
}

function shuffled<T>(items: readonly T[], rng: Rng): T[] {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = rng.int(0, i);
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

interface Group {
  id: string;
  parent: Group | null;
  depth: number;
  title: string;
  icon: boolean;
}

interface Service {
  id: string;
  index: number;
  group: Group | null;
  icon: string | null;
  title: string | null;
}

interface Junction {
  id: string;
  group: Group | null;
}

/** An edge endpoint: a service or a junction. Groups are never endpoints (the parser rejects it). */
interface Endpoint {
  id: string;
  group: Group | null;
  isService: boolean;
}

function generateOne(rng: Rng, index: number): GeneratedArchitecture {
  const config: ArchitectureConfig = {
    seed: rng.int(1, 9),
    randomize: rng.chance(0.25),
    nodeSeparation: rng.pick([75, 150]),
    numIter: rng.pick([200, 2500]),
    idealEdgeLengthMultiplier: rng.pick([1, 2, 3]),
    edgeElasticity: rng.pick([0.2, 0.45, 0.6]),
  };

  // About one diagram in twenty is a single service with no group, junction or edge.
  if (rng.chance(0.05)) {
    const icon = rng.chance(0.2) ? null : rng.pick(ICONS);
    const text = ['architecture-beta', `    service s1${icon ? `(${icon})` : ''}[Service 1]`].join(
      '\n'
    );
    return {
      index,
      text,
      config,
      shape: { groups: 0, maxDepth: 0, services: 1, junctions: 0, edges: 0, alignments: 0 },
    };
  }

  // Groups: a random nesting depth of 1 to 3 for the diagram, with 0 to 7 groups. A group
  // becomes a child of an earlier group that is still above the depth limit, or stays top level.
  const maxDepth = rng.int(1, 3);
  const groupCount = rng.chance(0.12) ? 0 : rng.int(1, 7);
  const groups: Group[] = [];
  for (let g = 1; g <= groupCount; g++) {
    const candidates = groups.filter((group) => group.depth < maxDepth);
    const parent = candidates.length > 0 && rng.chance(0.6) ? rng.pick(candidates) : null;
    groups.push({
      id: `g${g}`,
      parent,
      depth: (parent?.depth ?? 0) + 1,
      title: `Group ${g}`,
      icon: !rng.chance(0.1),
    });
  }
  const deepest = groups.reduce((max, group) => Math.max(max, group.depth), 0);

  // Services: 2 to 25. Most sit in a random group; the rest are top level.
  const serviceCount = rng.int(2, 25);
  const services: Service[] = [];
  for (let s = 1; s <= serviceCount; s++) {
    const group = groups.length > 0 && rng.chance(0.85) ? rng.pick(groups) : null;
    services.push({
      id: `s${s}`,
      index: s,
      group,
      icon: rng.chance(0.2) ? null : rng.pick(ICONS),
      title: rng.chance(0.9) ? `Service ${s}` : null,
    });
  }

  // Junctions: none in most diagrams, 1 to 3 in the rest.
  const junctionCount = rng.chance(0.3) ? rng.int(1, 3) : 0;
  const junctions: Junction[] = [];
  for (let j = 1; j <= junctionCount; j++) {
    const group = groups.length > 0 && rng.chance(0.8) ? rng.pick(groups) : null;
    junctions.push({ id: `j${j}`, group });
  }

  const lines: string[] = ['architecture-beta'];
  for (const group of groups) {
    const icon = group.icon ? '(cloud)' : '';
    const parent = group.parent ? ` in ${group.parent.id}` : '';
    lines.push(`    group ${group.id}${icon}[${group.title}]${parent}`);
  }
  for (const service of services) {
    const icon = service.icon ? `(${service.icon})` : '';
    const title = service.title ? `[${service.title}]` : '';
    const inGroup = service.group ? ` in ${service.group.id}` : '';
    lines.push(`    service ${service.id}${icon}${title}${inGroup}`);
  }
  for (const junction of junctions) {
    const inGroup = junction.group ? ` in ${junction.group.id}` : '';
    lines.push(`    junction ${junction.id}${inGroup}`);
  }

  // Edges: none in about one diagram in ten, otherwise up to one more than the number of endpoints.
  // Every unordered pair appears at most once, so no edge id is repeated.
  const endpoints: Endpoint[] = [
    ...services.map((s) => ({ id: s.id, group: s.group, isService: true })),
    ...junctions.map((j) => ({ id: j.id, group: j.group, isService: false })),
  ];
  const edgeTarget = endpoints.length < 2 || rng.chance(0.1) ? 0 : rng.int(1, endpoints.length + 3);
  const seenPairs = new Set<string>();
  let edgeCount = 0;
  for (let e = 0; e < edgeTarget; e++) {
    const a = rng.pick(endpoints);
    const b = rng.pick(endpoints);
    const key = [a.id, b.id].sort().join('|');
    if (a === b || seenPairs.has(key)) {
      continue;
    }
    seenPairs.add(key);
    const sideA = rng.pick(SIDES);
    const sideB = rng.chance(0.5) ? OPPOSITE[sideA] : rng.pick(SIDES);
    // A `{group}` modifier means the edge leaves its group. architectureDb.ts (addEdge) rejects one
    // when both endpoints are in the same group, so the modifier is only written across two groups.
    const sameGroup = a.group !== null && a.group === b.group;
    const lhs = `${a.id}${a.isService && a.group && !sameGroup && rng.chance(0.2) ? '{group}' : ''}`;
    const rhs = `${b.id}${b.isService && b.group && !sameGroup && rng.chance(0.2) ? '{group}' : ''}`;
    const form = rng.pick(['plain', 'plain', 'plain', 'arrow', 'into', 'label'] as const);
    const operator =
      form === 'arrow'
        ? ' --> '
        : form === 'into'
          ? ' <-- '
          : form === 'label'
            ? ` -[Link ${e}]- `
            : ' -- ';
    lines.push(`    ${lhs}:${sideA}${operator}${sideB}:${rhs}`);
    edgeCount++;
  }

  // Alignment hints: none in most diagrams. Members are services of one group, 2 or 3 of them, and
  // no service takes part twice along the same axis. Their order follows the declaration order most
  // of the time, but not always, so some chains contradict the edges and fail to render.
  const hintCount = rng.chance(0.4) ? rng.int(1, 2) : 0;
  let hintsMade = 0;
  const usedAlong: Record<'row' | 'column', Set<string>> = {
    row: new Set<string>(),
    column: new Set<string>(),
  };
  for (let h = 0; h < hintCount; h++) {
    const direction = rng.pick(['row', 'column'] as const);
    const used = usedAlong[direction];
    const eligible = groups.filter(
      (group) => services.filter((s) => s.group === group && !used.has(s.id)).length >= 2
    );
    if (eligible.length === 0) {
      break;
    }
    const group = rng.pick(eligible);
    const members = services.filter((s) => s.group === group && !used.has(s.id));
    const size = rng.int(2, Math.min(3, members.length));
    const chosen = shuffled(members, rng).slice(0, size);
    const ordered = rng.chance(0.75) ? [...chosen].sort((x, y) => x.index - y.index) : chosen;
    for (const member of ordered) {
      used.add(member.id);
    }
    lines.push(`    align ${direction} ${ordered.map((s) => s.id).join(' ')}`);
    hintsMade++;
  }

  return {
    index,
    text: lines.join('\n'),
    config,
    shape: {
      groups: groups.length,
      maxDepth: deepest,
      services: services.length,
      junctions: junctions.length,
      edges: edgeCount,
      alignments: hintsMade,
    },
  };
}

/** The first `count` diagrams of the stream started from `seed`. The same arguments give the same output. */
export function generateArchitectures(seed: number, count: number): GeneratedArchitecture[] {
  const rng = rngFrom(seed);
  const diagrams: GeneratedArchitecture[] = [];
  for (let index = 0; index < count; index++) {
    diagrams.push(generateOne(rng, index));
  }
  return diagrams;
}
