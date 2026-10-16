/**
 * Geometry comparison of the diagram types with one generic rule: the reference and ferrigura render
 * each fixture, both SVGs are measured in one Chromium page at scale 1, and the elements are paired
 * by their structural key and compared (`diagram-compare.ts`).
 *
 * The types are the P1 diagrams that ferrigura does not render yet (sequence, class, state, er,
 * gantt, timeline, gitgraph, journey, info), mapped to their fixture folders under
 * `e2e/diagrams`. Pie and flowchart are included so that the generic rule can be checked against
 * the specialised harnesses (`pie-geometry.ts`, `flowchart-geometry.ts`). Ferrigura renders none of
 * the others yet, so their fixtures are "not rendered" until it does.
 *
 * Both sides get the patched text (`patchText` in `common.ts`): the forced directive, and the
 * fallback theme. Each SVG is measured at scale 1. Excluded fixtures are those with `handDrawn`, a
 * non-layered layout, or CJK text; they are not compared.
 *
 * The measurement walks every element below the root, skipping `style`, `defs`, `marker` (with
 * their contents), `title` and `desc`. Each element gets its structural key, the chain of
 * `tag.class1.class2[n]` from the root, where `n` counts the earlier siblings with the same tag and
 * classes; the classes are sorted, and ids are recorded but not part of the key. Elements with
 * geometry: a `g` has its origin in viewBox units (its translation); rect, circle, ellipse, line,
 * polygon, polyline, image and text have their box (`getBBox`, mapped to viewBox units); a text has
 * its content; a path has its length and 17 samples along it (`getPointAtLength`).
 *
 * Tolerance: 0.05 px for positions and sizes, the viewBox and the max-width; text content and the
 * set of keys must match exactly. A key on one side only is a problem of its own. The worst
 * difference is reported as a percentage of the tolerance, per fixture.
 *
 * Output: the class of each fixture (within, outside, not rendered, excluded, reference failed),
 * the first problems, a summary per class and per fixture folder, and
 * `target/geometry/generic-<type>-geometry.json`. The specialised harnesses write their own files
 * (`pie-geometry.json`, `flowchart-geometry.json`), which this harness never touches.
 *
 * `--self` compares the reference with a second render of the same text, under the same key rule.
 * Every fixture of the type is compared, so the exclusions do not apply, and a difference is a
 * nondeterminism of the reference. Both renders get the same clock and random seed (`renderReference`
 * in `common.ts`), which takes the gantt today marker, gitGraph ids and rough.js strokes out of the
 * differences. Its summary goes to `target/geometry/generic-<type>-self-geometry.json`. Ferrigura is
 * not run.
 *
 * `--capture` writes, under `target/geometry/generic/<type>/<slug>/`, the reference's SVG and its
 * geometry (`geometry.json`), and, when the reference makes ELK calls, the ELK input and output. The
 * flowchart harness's own capture under `target/geometry/flowchart/` is not touched.
 *
 * Usage:
 *   pnpm port:geometry <type> [--filter PATTERN] [--images DIR] [--capture] [--self]
 *   DUMP=SLUG[,SLUG...] pnpm port:geometry <type>   writes target/geometry/dump-SLUG.json
 *
 * `<type>`: sequence, class, state, er, gantt, timeline, gitgraph, journey, info, pie, flowchart.
 * `--filter` takes a substring, or a glob with `*`, `**` and `?`, of the fixture's path under
 * `e2e/diagrams`, for example `class-diagram/**` or `state-diagram-v2/*note*`.
 *
 * `FERRIGURA_RENDER` replaces the ferrigura `render` example, as in the other harnesses.
 *
 * Exit code 1 only if a fixture rendered on both sides is outside the tolerance (for `--self`,
 * whenever the two reference renders differ).
 */
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import {
  buildRenderExample,
  countClasses,
  exclusionReason,
  flagValue,
  matchesFilter,
  measureSvg,
  mmdFiles,
  patchText,
  PATH_SAMPLES,
  PIXEL_TOLERANCE,
  renderFerrigura,
  renderReference,
  root,
  slugOf,
  withPage,
  workDir,
  writeJson,
  writeSideBySide,
  type FixtureClass,
  type FixtureResult,
} from './common.js';
import { compareDiagrams, type DiagramGeometry } from './diagram-compare.js';

const fixtureRoot = join(root, 'e2e/diagrams');

/** The fixture folders of each type, under `e2e/diagrams`. */
const TYPE_FOLDERS: Record<string, string[]> = {
  sequence: ['sequence'],
  class: ['class-diagram'],
  state: ['state-diagram', 'state-diagram-v2'],
  er: ['er-diagram'],
  gantt: ['gantt'],
  timeline: ['timeline'],
  gitgraph: ['git'],
  journey: ['journey'],
  info: ['info'],
  pie: ['pie'],
  flowchart: ['flowchart'],
};

/**
 * The page-side measurement of one SVG, as source text for the page (`pageHtml` in `common.ts`).
 * `record` gives one element its entry; `walk` visits the children of an element, in document
 * order, with the structural key of the parent.
 */
const MEASURE = `
const SKIPPED = new Set(['style', 'defs', 'marker', 'title', 'desc']);
const BOXED = new Set(['rect', 'circle', 'ellipse', 'line', 'polygon', 'polyline', 'image', 'text']);

const measure = (svg) => {
  const record = (element, key) => {
    const tag = element.tagName;
    const entry = {
      key: key,
      tag: tag,
      id: element.id || null,
      transform: null,
      origin: null,
      box: null,
      text: null,
      length: null,
      samples: null,
    };
    if (tag === 'g') {
      entry.transform = element.getAttribute('transform');
      const matrix = userMatrix(svg, element);
      entry.origin = matrix ? mapPoint(svg, matrix, 0, 0) : null;
    }
    if (BOXED.has(tag)) entry.box = boxOf(svg, element);
    if (tag === 'text') entry.text = element.textContent;
    if (tag === 'path') {
      const total = element.getTotalLength();
      const matrix = userMatrix(svg, element);
      entry.length = total;
      if (matrix) {
        try {
          const samples = [];
          for (let index = 0; index <= ${PATH_SAMPLES}; index++) {
            const point = element.getPointAtLength((total * index) / ${PATH_SAMPLES});
            samples.push(mapPoint(svg, matrix, point.x, point.y));
          }
          entry.samples = samples;
        } catch {
          // A path without segments has no points to sample: it stays without samples.
          entry.samples = null;
        }
      }
    }
    return entry;
  };
  const elements = [];
  const walk = (parent, parentKey) => {
    const seen = new Map();
    for (const child of parent.children) {
      if (SKIPPED.has(child.tagName)) continue;
      const signature = child.tagName + [...child.classList].sort().map((name) => '.' + name).join('');
      const index = seen.get(signature) ?? 0;
      seen.set(signature, index + 1);
      const key = parentKey + '/' + signature + '[' + index + ']';
      elements.push(record(child, key));
      walk(child, key);
    }
  };
  walk(svg, 'svg');
  return {
    id: svg.id,
    viewBox: numbers(svg.getAttribute('viewBox')),
    roledescription: svg.getAttribute('aria-roledescription'),
    elements: elements,
  };
};
`;

interface Options {
  type: string;
  filter: string | null;
  imageDir: string | null;
  capture: boolean;
  self: boolean;
}

function parseOptions(argv: string[]): Options {
  const name = (argv[0] ?? '').toLowerCase();
  if (argv[0] === undefined || argv[0].startsWith('--') || !(name in TYPE_FOLDERS)) {
    throw new Error(
      `usage: pnpm port:geometry <type> [--filter PATTERN] [--images DIR] [--capture] [--self] (types: ${Object.keys(TYPE_FOLDERS).join(', ')})`
    );
  }
  return {
    type: name,
    filter: flagValue(argv, '--filter'),
    imageDir: flagValue(argv, '--images'),
    capture: argv.includes('--capture'),
    self: argv.includes('--self'),
  };
}

/** One fixture's result: the shared fields, the reference's type and sizes. */
interface DiagramResult extends FixtureResult {
  /** The reference's `aria-roledescription`: the diagram it drew. */
  roledescription: string | null;
  /** The number of elements below the root in the reference. */
  elements: number;
  /** The number of ELK calls of the reference's render. */
  elkCalls: number;
}

/** The fixtures of a type, as paths under `e2e/diagrams` (folder first). */
async function fixturesOf(type: string): Promise<string[]> {
  const files: string[] = [];
  for (const folder of TYPE_FOLDERS[type]) {
    for (const file of await mmdFiles(join(fixtureRoot, folder))) {
      files.push(`${folder}/${file}`);
    }
  }
  return files;
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const options = parseOptions(argv);
  // DUMP=<slug>[,<slug>...] writes target/geometry/dump-<slug>.json for each fixture named.
  const dumpSlugs = new Set((process.env.DUMP ?? '').split(',').filter(Boolean));
  if (!options.self) buildRenderExample();

  const all = await fixturesOf(options.type);
  const slugs = new Map<string, string>();
  for (const file of all) {
    const slug = slugOf(file);
    if (slugs.has(slug))
      throw new Error(`fixtures ${slugs.get(slug)} and ${file} share the id ${slug}`);
    slugs.set(slug, file);
  }
  const selected = all.filter(
    (file) => options.filter === null || matchesFilter(file, options.filter)
  );
  const mode = options.self ? 'self' : 'ferrigura';
  console.log(
    `${options.type} geometry (${mode}): ${selected.length} of ${all.length} fixtures selected`
  );

  const results: DiagramResult[] = [];
  await withPage(MEASURE, async (page) => {
    for (const file of selected) {
      const slug = slugOf(file);
      const id = `ref-${slug}`;
      const source = await readFile(join(fixtureRoot, file), 'utf8');
      const text = patchText(source);
      const result: DiagramResult = {
        fixture: file,
        id,
        class: 'reference failed',
        reason: null,
        worstPercent: null,
        problems: [],
        warnings: [],
        roledescription: null,
        elements: 0,
        elkCalls: 0,
      };
      results.push(result);

      const clock = { now: Date.now(), seed: 1 };
      const reference = await renderReference(page, id, text, clock);
      result.elkCalls = reference.elk.length;
      if (!reference.ok || reference.svg === null) {
        result.reason = reference.error;
        console.log(`! ${file}  reference failed: ${reference.error}`);
        continue;
      }
      const referenceGeometry = await measureSvg<DiagramGeometry>(page, reference.svg);
      result.roledescription = referenceGeometry.roledescription;
      result.elements = referenceGeometry.elements.length;

      // The other side: a second render of the same text, or ferrigura's SVG.
      const otherName = options.self ? 'the second reference render' : 'ferrigura';
      let other: { svg: string; geometry: DiagramGeometry } | null = null;
      const excluded = options.self ? null : exclusionReason(source);
      if (excluded !== null) {
        result.class = 'excluded';
        result.reason = excluded;
        console.log(`– ${file}  excluded: ${excluded}`);
      } else if (options.self) {
        const second = await renderReference(page, `self-${slug}`, text, clock);
        if (!second.ok || second.svg === null) {
          result.class = 'outside';
          result.problems = [`the second reference render failed: ${second.error}`];
          console.log(`✗ ${file}  the second reference render failed: ${second.error}`);
          continue;
        }
        other = {
          svg: second.svg,
          geometry: await measureSvg<DiagramGeometry>(page, second.svg),
        };
      } else {
        const rendered = renderFerrigura(text, id);
        if ('error' in rendered) {
          result.class = 'not rendered';
          result.reason = rendered.error;
          console.log(`· ${file}  not rendered: ${rendered.error}`);
        } else {
          other = {
            svg: rendered.svg,
            geometry: await measureSvg<DiagramGeometry>(page, rendered.svg),
          };
        }
      }

      if (other !== null) {
        const { worst, problems } = compareDiagrams(referenceGeometry, other.geometry, otherName);
        result.class = problems.length === 0 ? 'within' : 'outside';
        result.worstPercent = worst === null ? null : worst * 100;
        result.problems = problems;
        const mark = result.class === 'within' ? '✓' : '✗';
        const worstText =
          worst === null
            ? 'no numeric comparison'
            : `worst ${(worst * 100).toFixed(1)}% of tolerance`;
        console.log(`${mark} ${file}  ${worstText}`);
        for (const problem of problems.slice(0, 5)) console.log(`    ${problem}`);
        if (options.imageDir) {
          await writeSideBySide(
            page,
            reference.svg,
            other.svg,
            join(options.imageDir, `${options.type}-${slug}.png`)
          );
        }
      }

      if (dumpSlugs.has(slug)) {
        await writeJson(join(workDir, `dump-${slug}.json`), {
          fixture: file,
          id,
          text,
          reference: { svg: reference.svg, geometry: referenceGeometry, elk: reference.elk },
          other: other && { side: otherName, svg: other.svg, geometry: other.geometry },
        });
      }
      if (options.capture) {
        const directory = join(workDir, 'generic', options.type, slug);
        await mkdir(directory, { recursive: true });
        await writeFile(join(directory, 'reference.svg'), reference.svg);
        await writeJson(join(directory, 'geometry.json'), referenceGeometry);
        if (reference.elk.length > 0) {
          await writeJson(
            join(directory, 'elk-input.json'),
            reference.elk.map((call) => call.input)
          );
          await writeJson(
            join(directory, 'elk-output.json'),
            reference.elk.map((call) => call.output)
          );
        }
      }
    }
  });

  // The summary: per class, the reasons of the excluded and the not rendered, and per folder.
  const counts = countClasses(results);
  const excludedBy = new Map<string, number>();
  const notRenderedBy = new Map<string, number>();
  for (const result of results) {
    if (result.class === 'excluded' && result.reason) {
      const key = result.reason.replace(/:.*$/, '');
      excludedBy.set(key, (excludedBy.get(key) ?? 0) + 1);
    }
    if (result.class === 'not rendered' && result.reason) {
      notRenderedBy.set(result.reason, (notRenderedBy.get(result.reason) ?? 0) + 1);
    }
  }
  const byFolder: Record<string, Record<FixtureClass, number>> = {};
  for (const folder of TYPE_FOLDERS[options.type]) {
    byFolder[folder] = countClasses(
      results.filter((result) => result.fixture.startsWith(`${folder}/`))
    );
  }
  const summary = {
    fixtures: results.length,
    ...counts,
    excludedBy: Object.fromEntries(excludedBy),
    notRenderedBy: Object.fromEntries(notRenderedBy),
    byFolder,
  };
  const outputName = options.self
    ? `generic-${options.type}-self-geometry.json`
    : `generic-${options.type}-geometry.json`;
  await writeJson(join(workDir, outputName), {
    type: options.type,
    mode,
    folders: TYPE_FOLDERS[options.type],
    tolerances: { pixels: PIXEL_TOLERANCE },
    summary,
    results,
  });

  console.log(`\n${options.type} geometry (${mode}): ${results.length} fixtures`);
  console.log(`  within tolerance:    ${counts.within}`);
  console.log(`  outside tolerance:   ${counts.outside}`);
  console.log(
    `  not rendered:        ${counts['not rendered']} (${[...notRenderedBy].map(([reason, n]) => `${n} x ${reason}`).join('; ') || 'none'})`
  );
  console.log(
    `  excluded:            ${counts.excluded} (${[...excludedBy].map(([reason, n]) => `${n} x ${reason}`).join('; ') || 'none'})`
  );
  console.log(`  reference failed:    ${counts['reference failed']}`);
  for (const [folder, folderCounts] of Object.entries(byFolder)) {
    console.log(
      `  ${folder}: within ${folderCounts.within}, outside ${folderCounts.outside}, not rendered ${folderCounts['not rendered']}, excluded ${folderCounts.excluded}, reference failed ${folderCounts['reference failed']}`
    );
  }
  console.log(`  written to ${join(workDir, outputName)}`);
  if (counts.outside > 0) process.exit(1);
}

try {
  await main();
} catch (error) {
  // Usage errors (an unknown type, a bad flag) print one line, not a stack.
  console.error((error as Error).message);
  process.exit(2);
}
