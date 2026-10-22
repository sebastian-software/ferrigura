/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Differential corpus generator for the ferrigura-venn port.
 *
 * For every input the venn renderer hands the layout library, it records what the library returns
 * in Chromium: the stages of the layout (the areas with their missing pairs, the greedy placement,
 * the constrained layout), the chart's circles and text centres (`VennDiagram`), and `layout()`'s
 * text centres and circles, with the random numbers each call drew.
 *
 * 1. The inputs. Each fixture under e2e/diagrams/venn is parsed by the reference's own parser and DB
 *    (through tools/port/reference.ts, as export-venn.ts does), and its renderSets are the reference's
 *    `ensurePairwiseSubsets` of the parsed subsets (vennRenderer.ts). The probes are written here, as
 *    venn texts or as areas handed to the library directly. The generated cases come from
 *    generate-set-systems.ts, and go through the same parser and renderer function.
 * 2. The reference. Chromium runs the harness page of tools/port/geometry/common.ts. The layout
 *    library's build (`build/venn.esm.js`) is served to the page, with one line appended that exports
 *    the functions the stages need (the build exports the rest). `Math.random` is replaced by the
 *    harness's seeded generator, which logs every draw.
 * 3. The chart is the library's own pipeline (`VennDiagram`, the build lines 1503-1526), replicated
 *    from its exported functions because the chart's drawing needs d3: the removal of zero-size
 *    singles, `venn`, `normalizeSolution` (orientation pi/2), `scaleSolution` (padding 15) and
 *    `computeTextCentres`. `layout()` is called as the library exports it.
 * 4. Each case is written to crates/ferrigura-venn/tests/corpus/<name>.json, with manifest.json. The
 *    directory is cleared of JSON first, so a re-run leaves no stale case.
 *
 * Run from the repository root:
 *
 *   pnpm exec tsx crates/ferrigura-venn/tools/capture-corpus.ts
 */
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { Page } from 'playwright';

import {
  provideDomForSanitising,
  openReference,
  registerDetectors,
  type Reference,
} from '../../../tools/port/reference.ts';
import { referenceRoot } from '../../../tools/port/reference-root.ts';
import { withPage } from '../../../tools/port/geometry/common.ts';
import {
  GENERATED_COUNT,
  GENERATOR_SEED,
  generateSetSystems,
  type GeneratedSetSystem,
} from './generate-set-systems.ts';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const CORPUS_DIR = join(REPO_ROOT, 'crates/ferrigura-venn/tests/corpus');
const FIXTURE_DIR = join(REPO_ROOT, 'e2e/diagrams/venn');

/** Version of the case and manifest format. Bump it when either changes shape. */
const CORPUS_VERSION = 1;
/** Indentation of the written JSON. */
const JSON_INDENT = 1;

/** The renderer's defaults (config.schema.yaml: width 800, height 450, padding 8). */
const DEFAULT_WIDTH = 800;
const DEFAULT_HEIGHT = 450;
const DEFAULT_LAYOUT_PADDING = 8;
/** The chart's padding: `VennDiagram` takes 15 when none is given (diagram.js 21-35). */
const CHART_PADDING = 15;
/** The title's height at the reference width (48 px scaled by width / 1600). */
const TITLE_HEIGHT = 48 * (DEFAULT_WIDTH / 1600);

/** The boxes the generated cases cycle through, with the layout's padding. */
const GENERATED_BOXES: [number, number, number][] = [
  [800, 450, 8],
  [1024, 640, 0],
  [640, 360, 15],
  [320, 240, 8],
];

// ─────────────────────────────────────────────────────────────────────────────
// Corpus format (read by the Rust tests in crates/ferrigura-venn/tests)
// ─────────────────────────────────────────────────────────────────────────────

/** A number as the corpus writes it: a JSON number, or the sentinel string for -0 and non-finite. */
type CorpusNumber = number | string;

interface CorpusArea {
  sets: string[];
  size: CorpusNumber;
}

interface CorpusCircle {
  id: string;
  x: CorpusNumber;
  y: CorpusNumber;
  radius: CorpusNumber;
}

interface CorpusCentre {
  x: CorpusNumber;
  y: CorpusNumber;
  disjoint: boolean;
}

interface CorpusPlaced {
  set: string;
  x: CorpusNumber;
  y: CorpusNumber;
  radius: CorpusNumber;
}

interface CorpusAreaLayout {
  sets: string[];
  text: CorpusCentre;
  circles: CorpusPlaced[];
}

type CaseGroup = 'fixture' | 'probe' | 'generated';

interface CorpusCase {
  name: string;
  group: CaseGroup;
  /** Repository path of the fixture, or how the probe or generated case was made. */
  source: string;
  description: string;
  /** The venn text, or null for an area list handed to the library directly. */
  text: string | null;
  input: {
    areas: CorpusArea[];
    width: number;
    height: number;
    chartPadding: number;
    layoutPadding: number;
  };
  /** The random numbers each step drew, in order. */
  draws: { stages: number[]; chart: number[]; layout: number[] };
  /** The stages of the chart's layout, on the areas the chart lays out (zero-size singles removed). */
  stages: {
    areas: CorpusArea[];
    greedy: CorpusCircle[] | null;
    constrained: CorpusCircle[] | null;
  } | null;
  chart: {
    venn: CorpusCircle[];
    normalized: CorpusCircle[];
    circles: CorpusCircle[];
    textCentres: ({ key: string } & CorpusCentre)[];
  } | null;
  layout: CorpusAreaLayout[] | null;
  /** The message of each step the library threw, by step. */
  errors: { greedy?: string; constrained?: string; chart?: string; layout?: string };
}

interface CorpusManifest {
  version: number;
  cases: { name: string; group: CaseGroup; source: string }[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Inputs
// ─────────────────────────────────────────────────────────────────────────────

/** An area as the renderer hands it to the library. */
interface Area {
  sets: string[];
  size: number;
}

/** One input of the run: its areas, the box, and the paddings. */
interface Job {
  name: string;
  group: CaseGroup;
  source: string;
  description: string;
  text: string | null;
  areas: Area[];
  width: number;
  height: number;
  layoutPadding: number;
}

/** The probes: hand-written texts, and area lists the library gets directly. */
const PROBE_TEXTS: { name: string; description: string; text: string }[] = [
  {
    name: 'numeric-ids-ascending',
    description:
      'Numeric set ids declared in ascending order, with unions: the key order of the library.',
    text: 'venn-beta\n  set "1"\n  set "2"\n  set "10"\n  union "1", "2"\n  union "2", "10"\n  union "1", "10"\n  union "1", "2", "10"',
  },
  {
    name: 'numeric-ids-declared-2-10-1',
    description:
      'The same numeric ids declared as 2, 10, 1, which the library reorders by its keys.',
    text: 'venn-beta\n  set "2":12\n  set "10":7\n  set "1":20\n  union "2", "10":3\n  union "1", "2":6\n  union "1", "10":2',
  },
  {
    name: 'alphanumeric-ids-ascending',
    description: 'Alphanumeric ids a1, a2, a10 declared in order, with a triple union.',
    text: 'venn-beta\n  set a1\n  set a2\n  set a10\n  union a1, a2\n  union a2, a10\n  union a1, a2, a10',
  },
  {
    name: 'alphanumeric-ids-declared-x2-x10-x1',
    description:
      'Alphanumeric ids declared as x2, x10, x1: the insertion order decides their place.',
    text: 'venn-beta\n  set x2:9\n  set x10:4\n  set x1:6\n  union x2, x10:2\n  union x10, x1:3\n  union x1, x2',
  },
  {
    name: 'four-sets-no-unions',
    description: 'Four sets and no unions: the constrained layout runs on the pairs alone.',
    text: 'venn-beta\n  set A\n  set B\n  set C\n  set D',
  },
  {
    name: 'four-sets-sizes-and-unions',
    description: 'Four sets with sizes and pairwise and triple unions.',
    text: 'venn-beta\n  set A:20\n  set B:15\n  set C:10\n  set D:8\n  union A, B:5\n  union B, C:3\n  union C, D:2\n  union A, D:4\n  union A, B, C:1',
  },
  {
    name: 'five-sets-triple-unions',
    description: 'Five sets with several triple unions.',
    text: 'venn-beta\n  set A:12\n  set B:9\n  set C:7\n  set D:5\n  set E:3\n  union A, B, C:2\n  union B, C, D:1.5\n  union C, D, E:0.5\n  union A, E:1',
  },
  {
    name: 'six-sets-all-pairs',
    description:
      'Six sets with all their pairwise unions and no sizes: the default sizes of the parser.',
    text: 'venn-beta\n  set A\n  set B\n  set C\n  set D\n  set E\n  set F\n  union A, B\n  union A, C\n  union A, D\n  union A, E\n  union A, F\n  union B, C\n  union B, D\n  union B, E\n  union B, F\n  union C, D\n  union C, E\n  union C, F\n  union D, E\n  union D, F\n  union E, F',
  },
  {
    name: 'seven-sets-chain',
    description: 'Seven sets in a chain of pairwise unions.',
    text: 'venn-beta\n  set A:9\n  set B:8\n  set C:7\n  set D:6\n  set E:5\n  set F:4\n  set G:3\n  union A, B:4\n  union B, C:3\n  union C, D:2\n  union D, E:1\n  union E, F:0.5\n  union F, G:0.25',
  },
  {
    name: 'set-declared-twice',
    description: 'A set declared twice, so the layout has two single areas for it (ten areas).',
    text: 'venn-beta\n  set A:10\n  set B\n  set C\n  set A:12\n  union A, B\n  union B, C\n  union A, C',
  },
  {
    name: 'union-declared-three-times',
    description: 'One union declared three times with different sizes (eight areas).',
    text: 'venn-beta\n  set A\n  set B\n  set C\n  union A, B:2\n  union A, B:3\n  union A, B:4\n  union B, C',
  },
  {
    name: 'zero-size-single',
    description: 'A zero-size set: the chart leaves it out with its areas, the layout keeps it.',
    text: 'venn-beta\n  set A:0\n  set B:10\n  set C:6\n  union A, B:3\n  union B, C:2',
  },
  {
    name: 'all-zero-sizes',
    description: 'Every set and every overlap of size zero: nothing is drawn by the chart.',
    text: 'venn-beta\n  set A:0\n  set B:0\n  union A, B:0',
  },
  {
    name: 'single-set',
    description: 'A single set and nothing else.',
    text: 'venn-beta\n  set A',
  },
  {
    name: 'complete-overlap',
    description: 'The overlap equals the smaller set: the smaller circle lies inside the larger.',
    text: 'venn-beta\n  set A:10\n  set B:4\n  union A, B:4',
  },
  {
    name: 'overlap-exceeds-set',
    description: 'The overlap is larger than the smaller set, which the library takes as complete.',
    text: 'venn-beta\n  set A:10\n  set B:4\n  union A, B:9',
  },
  {
    name: 'disjoint-pairs',
    description: 'Four sets, every overlap zero.',
    text: 'venn-beta\n  set A:10\n  set B:10\n  set C:10\n  set D:10\n  union A, B:0\n  union A, C:0\n  union B, D:0',
  },
  {
    name: 'negative-overlap',
    description: 'A negative overlap: the library throws in its bisection.',
    text: 'venn-beta\n  set A:10\n  set B:10\n  union A, B:-5',
  },
  {
    name: 'quoted-ids-with-spaces',
    description: 'Quoted ids with spaces and punctuation.',
    text: 'venn-beta\n  set "Front end":6\n  set "Back end":5\n  set "Dev Ops":4\n  union "Front end", "Back end":2\n  union "Back end", "Dev Ops":1',
  },
  {
    name: 'triple-union-only',
    description: 'Three sets with only the triple union: seven areas, so the greedy layout alone.',
    text: 'venn-beta\n  set Desirable\n  set Feasible\n  set Viable\n  union Desirable, Feasible, Viable',
  },
  {
    name: 'eight-areas-with-duplicate',
    description:
      'Three sets with a duplicate set and a pair: eight areas, so the constrained layout is compared.',
    text: 'venn-beta\n  set A:9\n  set B:6\n  set A:7\n  set C:5\n  union A, B:2\n  union B, C:1',
  },
  {
    name: 'constructor-set-name',
    description: 'A set named constructor, an Object.prototype name the library sees as present.',
    text: 'venn-beta\n  set constructor:8\n  set B:6\n  set C:4\n  union constructor, B:2\n  union B, C:1',
  },
  {
    name: 'proto-set-name',
    description: 'A set named __proto__: the port refuses it (Error::ReservedName).',
    text: 'venn-beta\n  set __proto__:8\n  set B:6\n  union __proto__, B:2',
  },
];

/** Probes that hand the library an area list directly, which the diagram's parser would not give. */
const PROBE_AREAS: { name: string; description: string; areas: Area[] }[] = [
  {
    name: 'no-areas',
    description: 'An empty list: the library has no set to start from.',
    areas: [],
  },
  {
    name: 'union-without-single',
    description:
      'A two-set area whose second set has no single-set area: the library throws reading its circle.',
    areas: [
      { sets: ['A'], size: 10 },
      { sets: ['A', 'B'], size: 2 },
    ],
  },
  {
    name: 'empty-area',
    description: 'An area with no set.',
    areas: [
      { sets: ['A'], size: 10 },
      { sets: [], size: 1 },
    ],
  },
];

/** Reads a fixture's text. The file is the diagram's text, front matter and all. */
function fixtureFiles(dir: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir).sort()) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      files.push(...fixtureFiles(path));
    } else if (entry.endsWith('.mmd')) {
      files.push(path);
    }
  }
  return files;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\da-z]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

// ─────────────────────────────────────────────────────────────────────────────
// The reference's parser, DB and renderer function
// ─────────────────────────────────────────────────────────────────────────────

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: { encodeEntities(text: string): string };
  grammar: { yy: unknown; parse(text: string): unknown };
  db: {
    db: {
      clear(): void;
      setDiagramTitle(text: string): void;
      getDiagramTitle(): string;
      getSubsetData(): { sets: string[]; size: number }[];
    };
  };
  renderer: {
    ensurePairwiseSubsets(
      subsets: { sets: string[]; size: number }[]
    ): { sets: string[]; size: number }[];
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, grammar, db, renderer] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/venn/parser/venn.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/venn/vennDB.ts'),
    reference.load<Modules['renderer']>('/packages/mermaid/src/diagrams/venn/vennRenderer.ts'),
  ]);
  return { preprocess, utils, grammar: grammar.parser, db, renderer };
}

/**
 * The renderer's `renderSets` for a text: the parser's subsets, with the synthetic pairs of
 * `ensurePairwiseSubsets`. Returns the title too, which sets the box height.
 */
function renderSetsOf(modules: Modules, text: string): { areas: Area[]; title: string } {
  const { code, title } = modules.preprocess.preprocessDiagram(text);
  const vennDb = modules.db.db;
  vennDb.clear();
  (modules.grammar as { yy: unknown }).yy = vennDb;
  if (title) {
    vennDb.setDiagramTitle(title);
  }
  modules.grammar.parse(modules.utils.encodeEntities(code.cleaned) + '\n');
  const subsets = vennDb
    .getSubsetData()
    .map((subset) => ({ sets: subset.sets, size: subset.size }));
  const areas = modules.renderer.ensurePairwiseSubsets(subsets).map((area) => ({
    sets: [...area.sets],
    size: area.size,
  }));
  return { areas, title: vennDb.getDiagramTitle() };
}

/** A number as the corpus writes it (see CorpusNumber). */
function encodeNumber(value: number): CorpusNumber {
  if (Object.is(value, -0)) {
    return '-0';
  }
  if (Number.isNaN(value)) {
    return 'NaN';
  }
  if (value === Infinity) {
    return 'Infinity';
  }
  if (value === -Infinity) {
    return '-Infinity';
  }
  return value;
}

function encodeArea(area: Area): CorpusArea {
  return { sets: area.sets, size: encodeNumber(area.size) };
}

// ─────────────────────────────────────────────────────────────────────────────
// The browser side: the library in Chromium, run by the harness page
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The runner of the reference, as plain JavaScript evaluated in the page. It takes the jobs and
 * returns the records: the stages, the chart and `layout()`, the draws of each step, and the
 * messages of the steps that throw. See the module comment for the order of the calls.
 */
const BROWSER_RUNNER = String.raw`async (jobs) => {
  const venn = await import('/venn/venn.esm.js');
  const enc = (value) =>
    Object.is(value, -0) ? '-0' : Number.isNaN(value) ? 'NaN' : value === Infinity ? 'Infinity' : value === -Infinity ? '-Infinity' : value;
  const encCircle = (c) => ({ x: enc(c.x), y: enc(c.y), radius: enc(c.radius) });
  const encCentre = (c) => ({ x: enc(c.x), y: enc(c.y), disjoint: c.disjoint === true });
  const circlesOf = (object) => Object.keys(object).map((id) => ({ id, ...encCircle(object[id]) }));
  const encArea = (area) => ({ sets: area.sets, size: enc(area.size) });
  const messageOf = (error) => (error && error.message !== undefined ? String(error.message) : String(error));
  // The harness's generator (tools/port/geometry/common.ts, renderReference), logging each draw.
  const seeded = (seed) => {
    let state = seed >>> 0;
    return () => {
      state = (state + 0x6d2b79f5) | 0;
      let t = Math.imul(state ^ (state >>> 15), 1 | state);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  };
  const realRandom = Math.random;
  let generator = seeded(1);
  let sink = null;
  Math.random = () => {
    const value = generator();
    if (sink) sink.push(value);
    return value;
  };
  const outputs = [];
  try {
    for (const job of jobs) {
      const areas = job.areas;
      const record = { stages: null, chart: null, layout: null, draws: { stages: [], chart: [], layout: [] }, errors: {} };

      // The chart's data: the zero-size singles and the areas that name them are left out.
      const zero = new Set();
      areas.forEach((datum) => {
        if (datum.size == 0 && datum.sets.length == 1) zero.add(datum.sets[0]);
      });
      const data = areas.filter((datum) => !datum.sets.some((set) => zero.has(set)));
      const parameters = () => ({ lossFunction: venn.lossFunction, distinct: false });

      // Stages: a generator of its own, on the chart's data.
      if (data.length > 0) {
        generator = seeded(1);
        sink = record.draws.stages;
        const withMissing = venn.addMissingAreas(data, {});
        const stages = {
          areas: withMissing.map(encArea),
          greedy: null,
          constrained: null,
        };
        try {
          stages.greedy = circlesOf(venn.greedyLayout(withMissing, parameters()));
        } catch (error) {
          record.errors.greedy = messageOf(error);
        }
        if (withMissing.length >= 8) {
          try {
            stages.constrained = circlesOf(
              venn.constrainedMDSLayout(withMissing, { maxIterations: 500, ...parameters() })
            );
          } catch (error) {
            record.errors.constrained = messageOf(error);
          }
        }
        record.stages = stages;
      }

      // The chart: the generator restarts at seed 1, as the render of the chart does.
      generator = seeded(1);
      sink = record.draws.chart;
      try {
        if (data.length > 0) {
          const raw = venn.venn(data, parameters());
          const rawCircles = circlesOf(raw);
          const normalized = venn.normalizeSolution(raw, Math.PI / 2, null);
          const normalizedCircles = circlesOf(normalized);
          const scaled = venn.scaleSolution(normalized, job.width, job.height, ${CHART_PADDING}, null);
          const scaledCircles = circlesOf(scaled);
          const texts = venn.computeTextCentres(scaled, data, false);
          record.chart = {
            venn: rawCircles,
            normalized: normalizedCircles,
            circles: scaledCircles,
            textCentres: Object.keys(texts).map((key) => ({ key, ...encCentre(texts[key]) })),
          };
        } else {
          record.chart = { venn: [], normalized: [], circles: [], textCentres: [] };
        }
      } catch (error) {
        record.errors.chart = messageOf(error);
      }

      // layout(): the generator continues from the chart's draws, unless the chart threw, when the
      // renderer's second call starts from a generator of its own (the case does not arise in the
      // corpus' fixtures, and the Rust test follows the same rule).
      if (record.errors.chart !== undefined) generator = seeded(1);
      sink = record.draws.layout;
      try {
        const laid = venn.layout(areas, {
          width: job.width,
          height: job.height,
          padding: job.layoutPadding,
        });
        record.layout = laid.map((area) => ({
          sets: area.data.sets,
          text: encCentre(area.text),
          circles: area.circles.map((c) => ({ set: c.set, ...encCircle(c) })),
        }));
      } catch (error) {
        record.errors.layout = messageOf(error);
      }
      sink = null;
      outputs.push(record);
    }
  } finally {
    Math.random = realRandom;
  }
  return outputs;
}`;

/** The layout library's build, with the functions the stages need exported (the build exports the rest). */
function libraryBuildSource(): string {
  // The build Mermaid bundles, which lives in node_modules/.pnpm (the mermaid package links to it).
  const build = realpathSync(
    join(referenceRoot, 'packages/mermaid/node_modules/@upsetjs/venn.js/build/venn.esm.js')
  );
  return `${readFileSync(build, 'utf8')}\nexport { addMissingAreas, constrainedMDSLayout };\n`;
}

/**
 * Runs the jobs in Chromium, through the harness page. The library's build is served at
 * /venn/venn.esm.js, by a route on the page, after the harness has loaded.
 */
async function runInBrowser(jobs: Job[]): Promise<Record<string, unknown>[]> {
  const payload = jobs.map((job) => ({
    areas: job.areas.map((area) => ({ sets: area.sets, size: area.size })),
    width: job.width,
    height: job.height,
    layoutPadding: job.layoutPadding,
  }));
  const source = libraryBuildSource();
  return withPage('', async (page: Page) => {
    await page.route('**/venn/venn.esm.js', (route) =>
      route.fulfill({ status: 200, contentType: 'text/javascript', body: source })
    );
    const expression = `(${BROWSER_RUNNER})(${JSON.stringify(payload)})`;
    return await page.evaluate(expression);
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Writing the corpus
// ─────────────────────────────────────────────────────────────────────────────

interface BrowserRecord {
  stages: {
    areas: CorpusArea[];
    greedy: CorpusCircle[] | null;
    constrained: CorpusCircle[] | null;
  } | null;
  chart: CorpusCase['chart'];
  layout: CorpusAreaLayout[] | null;
  draws: { stages: number[]; chart: number[]; layout: number[] };
  errors: CorpusCase['errors'];
}

function caseOf(job: Job, record: BrowserRecord): CorpusCase {
  return {
    name: job.name,
    group: job.group,
    source: job.source,
    description: job.description,
    text: job.text,
    input: {
      areas: job.areas.map(encodeArea),
      width: job.width,
      height: job.height,
      chartPadding: CHART_PADDING,
      layoutPadding: job.layoutPadding,
    },
    draws: record.draws,
    stages: record.stages,
    chart: record.chart,
    layout: record.layout,
    errors: record.errors,
  };
}

function writeCorpus(cases: CorpusCase[]): void {
  mkdirSync(CORPUS_DIR, { recursive: true });
  for (const file of readdirSync(CORPUS_DIR)) {
    if (file.endsWith('.json')) {
      rmSync(join(CORPUS_DIR, file));
    }
  }
  for (const record of cases) {
    writeFileSync(
      join(CORPUS_DIR, `${record.name}.json`),
      `${JSON.stringify(record, null, JSON_INDENT)}\n`
    );
  }
  const manifest: CorpusManifest = {
    version: CORPUS_VERSION,
    cases: cases.map((record) => ({
      name: record.name,
      group: record.group,
      source: record.source,
    })),
  };
  writeFileSync(
    join(CORPUS_DIR, 'manifest.json'),
    `${JSON.stringify(manifest, null, JSON_INDENT)}\n`
  );
}

async function main(): Promise<void> {
  const reference = await openReference();
  const jobs: Job[] = [];
  try {
    const modules = await loadModules(reference);

    for (const file of fixtureFiles(FIXTURE_DIR)) {
      const text = readFileSync(file, 'utf8');
      const { areas, title } = renderSetsOf(modules, text);
      const height = DEFAULT_HEIGHT - (title ? TITLE_HEIGHT : 0);
      jobs.push({
        name: `fixture-${slugify(relative(FIXTURE_DIR, file).replace(/\.mmd$/, ''))}`,
        group: 'fixture',
        source: relative(REPO_ROOT, file),
        description: 'A fixture of e2e/diagrams/venn, as the renderer lays it out.',
        text,
        areas,
        width: DEFAULT_WIDTH,
        height,
        layoutPadding: DEFAULT_LAYOUT_PADDING,
      });
    }

    for (const probe of PROBE_TEXTS) {
      const { areas, title } = renderSetsOf(modules, probe.text);
      jobs.push({
        name: `probe-${probe.name}`,
        group: 'probe',
        source: `probe: ${probe.name}`,
        description: probe.description,
        text: probe.text,
        areas,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT - (title ? TITLE_HEIGHT : 0),
        layoutPadding: DEFAULT_LAYOUT_PADDING,
      });
    }

    for (const probe of PROBE_AREAS) {
      jobs.push({
        name: `probe-${probe.name}`,
        group: 'probe',
        source: `probe: ${probe.name} (areas given directly)`,
        description: probe.description,
        text: null,
        areas: probe.areas,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        layoutPadding: DEFAULT_LAYOUT_PADDING,
      });
    }

    const generated: GeneratedSetSystem[] = generateSetSystems(GENERATED_COUNT, GENERATOR_SEED);
    generated.forEach((system, position) => {
      const { areas, title } = renderSetsOf(modules, system.text);
      const [width, height, layoutPadding] = GENERATED_BOXES[position % GENERATED_BOXES.length];
      jobs.push({
        name: `generated-${String(system.index).padStart(3, '0')}-${system.scheme}`,
        group: 'generated',
        source: `generated: seed ${GENERATOR_SEED}, index ${system.index}, ${system.setCount} sets, ${system.scheme} ids`,
        description: `A generated set system (${system.scheme} ids, ${system.setCount} sets).`,
        text: system.text,
        areas,
        width,
        height: height - (title ? TITLE_HEIGHT : 0),
        layoutPadding,
      });
    });
  } finally {
    await reference.close();
  }

  console.log(`${jobs.length} inputs; running the reference in Chromium`);
  const records = (await runInBrowser(jobs)) as unknown as BrowserRecord[];
  const cases = jobs.map((job, index) => caseOf(job, records[index]));
  writeCorpus(cases);

  const failed = cases.filter((record) => Object.keys(record.errors).length > 0);
  console.log(`wrote ${cases.length} cases to ${relative(REPO_ROOT, CORPUS_DIR)}`);
  console.log(`${failed.length} cases where the reference threw:`);
  for (const record of failed) {
    console.log(`  ${record.name}: ${JSON.stringify(record.errors)}`);
  }
}

await main();
