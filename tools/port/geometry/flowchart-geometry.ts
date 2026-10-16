/**
 * Geometry comparison of flowcharts: the JavaScript reference and ferrigura, for the fixtures
 * under `e2e/diagrams/flowchart` (all of them, or those picked by `--filter` and `--stage`).
 *
 * Both sides get the same text, patched by `patchText` in `common.ts`: the directive that forces
 * HTML labels off, the font to Lexica Ultralegible and the security level to strict, after the
 * front matter and again at the end, and the fallback theme for a theme ferrigura does not ship.
 * Look and theme are not forced otherwise: the flowchart defaults (look neo, theme redux-color)
 * apply on both sides, as they do for a fixture that configures nothing. The run also counts
 * `foreignObject` elements in the reference, which must be zero.
 *
 * The reference is Mermaid's built bundle (`packages/mermaid/dist`), run in Chromium. ferrigura
 * renders the same patched text with its `render` example, with `--id` set to the reference's
 * id, so both SVGs can be paired by element id. Both SVGs are measured in one Chromium page, in
 * their viewBox coordinates, at scale 1: the nodes, the edge paths (sampled), the edge labels, the
 * clusters, the root box and the element counts per class.
 *
 * Excluded fixtures are a class of their own, not failures: the effective text sets look
 * `handDrawn` (rough.js is out of scope), or a layout that is not layered (ferrigura falls back to
 * layered), or it has CJK text. A fixture that the reference renders and ferrigura does not is
 * "not rendered": the `render` example fails for every flowchart until ferrigura draws one.
 *
 * With `--capture`, the reference's SVG, its geometry and every ELK call of its render (input and
 * output, in order) are written under `target/geometry/flowchart/<slug>/`. The ELK input is the
 * reference data for the Rust side. If `ferrigura --example flowchart-elk` builds, its `elkInput`
 * is compared with the reference's input, in node and edge order, sizes, labels and layout options.
 *
 * Tolerance: 0.05 px for positions and sizes, as in the pie harness; counts and marker kinds must
 * match exactly. The worst difference is reported as a percentage of the tolerance.
 *
 * Usage:
 *   pnpm port:geometry-flowchart [--filter PATTERN] [--stage NAME[,NAME...]] [--images DIR] [--capture]
 *   DUMP=SLUG[,SLUG...] pnpm port:geometry-flowchart   writes target/geometry/dump-SLUG.json
 *
 * `FERRIGURA_RENDER` and `FERRIGURA_FLOWCHART_ELK` replace the ferrigura binaries (any program with
 * the same arguments); they exist to test the harness itself.
 *
 * Exit code 1 only if a fixture that rendered on both sides is outside the tolerance.
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
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
import {
  compareElkInput,
  compareGeometry,
  type ElkElement,
  type SvgGeometry,
} from './flowchart-compare.js';
import { STAGES } from './flowchart-stages.js';

const fixtureDir = join(root, 'e2e/diagrams/flowchart');
/**
 * The `flowchart-elk` example, which prints ferrigura's ELK input for a text. `FERRIGURA_FLOWCHART_ELK`
 * replaces it, as `FERRIGURA_RENDER` replaces the render example.
 */
const elkOverride = process.env.FERRIGURA_FLOWCHART_ELK;
const elkBinary = elkOverride ?? join(root, 'target/debug/examples/flowchart-elk');

/**
 * The measurement of one flowchart SVG, as source text for the page (`pageHtml` in `common.ts`):
 * the nodes, the edge paths sampled along their length, the edge labels, the clusters, the root
 * attributes and the element counts per class.
 */
const MEASURE = `
const SHAPES = 'rect, circle, ellipse, polygon, path';
const notInLabel = (element) => element.closest('g.label') === null;

const measure = (svg) => {
  // Nodes: one group per vertex, the centre is the group's origin in viewBox coordinates.
  const nodes = [...svg.querySelectorAll('g.nodes > g')].map((group) => {
    const labelShape = group.querySelector('.label-container');
    const fallback = [...group.querySelectorAll(SHAPES)].find(notInLabel) || null;
    const shape = labelShape || fallback;
    const label = group.querySelector('g.label');
    const groupMatrix = userMatrix(svg, group);
    return {
      id: group.id,
      transform: group.getAttribute('transform'),
      centre: groupMatrix ? mapPoint(svg, groupMatrix, 0, 0) : null,
      shape: {
        source: labelShape ? 'label-container' : fallback ? 'shape' : 'none',
        tag: shape ? shape.tagName : null,
        box: shape ? boxOf(svg, shape) : null,
      },
      label: label ? boxOf(svg, label) : null,
    };
  });
  const edges = [...svg.querySelectorAll('g.edgePaths > path')].map((path) => {
    const total = path.getTotalLength();
    const matrix = userMatrix(svg, path);
    const samples = [];
    for (let index = 0; index <= ${PATH_SAMPLES}; index++) {
      const point = path.getPointAtLength((total * index) / ${PATH_SAMPLES});
      samples.push(mapPoint(svg, matrix, point.x, point.y));
    }
    const raw = path.getAttribute('data-points');
    let dataPoints = null;
    if (raw !== null) {
      try {
        dataPoints = JSON.parse(atob(raw)).map((point) => ({ x: point.x, y: point.y }));
      } catch {
        dataPoints = 'invalid';
      }
    }
    return {
      id: path.id,
      d: path.getAttribute('d'),
      length: total,
      samples,
      dataPoints,
      markerStart: path.getAttribute('marker-start'),
      markerEnd: path.getAttribute('marker-end'),
    };
  });
  const edgeLabels = [...svg.querySelectorAll('g.label[data-id]')].map((label) => {
    const background = label.querySelector('rect.background');
    return {
      dataId: label.getAttribute('data-id'),
      box: boxOf(svg, label),
      background: background ? boxOf(svg, background) : null,
    };
  });
  const clusters = [...svg.querySelectorAll('g.clusters > g.cluster')].map((group) => {
    const rect = group.querySelector(':scope > rect') || group.querySelector('rect');
    const label = group.querySelector(':scope > g.cluster-label') || group.querySelector('g.cluster-label');
    return {
      id: group.id,
      rect: rect ? boxOf(svg, rect) : null,
      label: label ? boxOf(svg, label) : null,
    };
  });
  // Elements per class token: a structural check (counts must match).
  const counts = {};
  for (const element of svg.querySelectorAll('*')) {
    const classes = element.getAttribute('class');
    if (!classes) continue;
    for (const token of classes.split(/\\s+/)) {
      if (token) counts[token] = (counts[token] || 0) + 1;
    }
  }
  const looks = [...svg.querySelectorAll('[data-look]')]
    .map((element) => element.getAttribute('data-look'))
    .filter((look, index, all) => all.indexOf(look) === index)
    .sort();
  return {
    id: svg.id,
    viewBox: numbers(svg.getAttribute('viewBox')),
    width: svg.getAttribute('width'),
    height: svg.getAttribute('height'),
    style: svg.getAttribute('style'),
    roledescription: svg.getAttribute('aria-roledescription'),
    looks,
    foreignObjects: svg.querySelectorAll('foreignObject').length,
    nodes,
    edges,
    edgeLabels,
    clusters,
    counts,
  };
};
`;

/** One fixture's result: the shared fields, and the ELK comparison of the flowchart. */
interface FlowchartResult extends FixtureResult {
  elk: {
    status: 'not available' | 'not compared' | 'match' | 'differ';
    referenceCalls: number;
    problems: string[];
  };
}

interface Options {
  filter: string | null;
  stages: string[];
  imageDir: string | null;
  capture: boolean;
}

function parseOptions(argv: string[]): Options {
  const stage = flagValue(argv, '--stage');
  const stages = stage
    ? stage
        .split(',')
        .map((name) => name.trim())
        .filter(Boolean)
    : [];
  for (const name of stages) {
    if (!(name in STAGES)) {
      throw new Error(`unknown stage "${name}" (stages: ${Object.keys(STAGES).join(', ')})`);
    }
  }
  return {
    filter: flagValue(argv, '--filter'),
    stages,
    imageDir: flagValue(argv, '--images'),
    capture: argv.includes('--capture'),
  };
}

/** The `flowchart-elk` example's ELK input for the text, or the first line of its error. */
function runElkExample(text: string, id: string): { input: ElkElement } | { error: string } {
  const result = spawnSync(elkBinary, ['--id', id], {
    input: text,
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) return { error: result.error.message };
  if (result.status !== 0) {
    return { error: (result.stderr || `exit status ${result.status}`).trim().split('\n')[0] };
  }
  try {
    return { input: (JSON.parse(result.stdout) as { elkInput: ElkElement }).elkInput };
  } catch (error) {
    return { error: `output is not JSON: ${(error as Error).message}` };
  }
}

/** Whether the `flowchart-elk` example builds. Reported once if it does not. */
function elkExampleBuilds(): string | null {
  if (elkOverride !== undefined) return null;
  try {
    execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'flowchart-elk'], {
      cwd: root,
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    return null;
  } catch (error) {
    const stderr = (error as { stderr?: Buffer }).stderr?.toString() ?? '';
    return (
      stderr
        .trim()
        .split('\n')
        .find((line) => line.trim() !== '') ?? 'does not build'
    );
  }
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const options = parseOptions(argv);
  // DUMP=<slug>[,<slug>...] writes target/geometry/dump-<slug>.json for each fixture named.
  const dumpSlugs = new Set((process.env.DUMP ?? '').split(',').filter(Boolean));

  buildRenderExample();
  const elkProblem = elkExampleBuilds();
  if (elkProblem !== null) {
    console.log(
      `ELK stage input: stages not available (ferrigura --example flowchart-elk: ${elkProblem})`
    );
  }

  // Fixtures, keyed by their path relative to the flowchart folder.
  const all = await mmdFiles(fixtureDir);
  const slugs = new Map<string, string>();
  for (const file of all) {
    const slug = slugOf(file);
    if (slugs.has(slug))
      throw new Error(`fixtures ${slugs.get(slug)} and ${file} share the id ${slug}`);
    slugs.set(slug, file);
  }
  const inStages = new Set<string>();
  for (const name of options.stages) {
    for (const file of STAGES[name]) {
      if (!all.includes(file)) throw new Error(`stage ${name} lists ${file}, which does not exist`);
      inStages.add(file);
    }
  }
  const selected = all.filter(
    (file) =>
      (options.stages.length === 0 || inStages.has(file)) &&
      (options.filter === null || matchesFilter(file, options.filter))
  );
  console.log(`flowchart geometry: ${selected.length} of ${all.length} fixtures selected`);

  const results: FlowchartResult[] = [];
  let foreignObjectFixtures = 0;
  await withPage(MEASURE, async (page) => {
    for (const file of selected) {
      const slug = slugOf(file);
      const id = `ref-${slug}`;
      const fixtureSource = await readFile(join(fixtureDir, file), 'utf8');
      const text = patchText(fixtureSource);
      const result: FlowchartResult = {
        fixture: file,
        id,
        class: 'reference failed',
        reason: null,
        worstPercent: null,
        problems: [],
        warnings: [],
        elk: {
          status: elkProblem === null ? 'not compared' : 'not available',
          referenceCalls: 0,
          problems: [],
        },
      };
      results.push(result);

      const reference = await renderReference(page, id, text);
      result.elk.referenceCalls = reference.elk.length;
      if (!reference.ok || reference.svg === null) {
        result.reason = reference.error;
        console.log(`! ${file}  reference failed: ${reference.error}`);
        continue;
      }
      const referenceGeometry = await measureSvg<SvgGeometry>(page, reference.svg);
      if (referenceGeometry.roledescription !== 'flowchart-v2') {
        result.reason = `reference SVG is ${referenceGeometry.roledescription ?? 'not a diagram'}, not a flowchart`;
        console.log(`! ${file}  ${result.reason}`);
        continue;
      }
      if (referenceGeometry.foreignObjects > 0) {
        foreignObjectFixtures++;
        result.warnings.push(
          `reference draws ${referenceGeometry.foreignObjects} foreignObject element(s): the forced htmlLabels did not apply`
        );
      }

      const excluded = exclusionReason(fixtureSource);
      let ferrigura: { svg: string; geometry: SvgGeometry } | null = null;
      if (excluded !== null) {
        result.class = 'excluded';
        result.reason = excluded;
        console.log(`– ${file}  excluded: ${excluded}`);
      } else {
        const rendered = renderFerrigura(text, id);
        if ('error' in rendered) {
          result.class = 'not rendered';
          result.reason = rendered.error;
          console.log(`· ${file}  not rendered: ${rendered.error}`);
        } else {
          const geometry = await measureSvg<SvgGeometry>(page, rendered.svg);
          const { worst, problems } = compareGeometry(referenceGeometry, geometry);
          ferrigura = { svg: rendered.svg, geometry };
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
              rendered.svg,
              join(options.imageDir, `flowchart-${slug}.png`)
            );
          }
        }
      }

      if (dumpSlugs.has(slug)) {
        await writeJson(join(workDir, `dump-${slug}.json`), {
          fixture: file,
          id,
          text,
          reference: { svg: reference.svg, geometry: referenceGeometry, elk: reference.elk },
          ferrigura: ferrigura && { svg: ferrigura.svg, geometry: ferrigura.geometry },
        });
      }
      if (options.capture) {
        const directory = join(workDir, 'flowchart', slug);
        await mkdir(directory, { recursive: true });
        await writeFile(join(directory, 'reference.svg'), reference.svg);
        await writeJson(join(directory, 'reference-geometry.json'), referenceGeometry);
        await writeJson(
          join(directory, 'elk-input.json'),
          reference.elk.map((call) => call.input)
        );
        await writeJson(
          join(directory, 'elk-output.json'),
          reference.elk.map((call) => call.output)
        );
      }
      // ELK: the example's input for the same text, against the reference's first call.
      if (elkProblem === null && result.class !== 'excluded' && reference.elk.length > 0) {
        const example = runElkExample(text, id);
        if ('error' in example) {
          result.elk.status = 'not available';
          result.elk.problems = [example.error];
        } else {
          const comparison = compareElkInput(reference.elk[0].input as ElkElement, example.input);
          result.elk.status = comparison.status;
          result.elk.problems = comparison.problems;
        }
      }
    }
  });

  const count = (name: FixtureClass) => countClasses(results)[name];
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
  const summary = {
    fixtures: results.length,
    within: count('within'),
    outside: count('outside'),
    notRendered: count('not rendered'),
    excluded: count('excluded'),
    referenceFailed: count('reference failed'),
    excludedBy: Object.fromEntries(excludedBy),
    notRenderedBy: Object.fromEntries(notRenderedBy),
    foreignObjectFixtures,
  };
  const elkCompared = results.filter(
    (result) => result.elk.status === 'match' || result.elk.status === 'differ'
  );
  const elkMatch = results.filter((result) => result.elk.status === 'match').length;

  await writeJson(join(workDir, 'flowchart-geometry.json'), {
    tolerances: { pixels: PIXEL_TOLERANCE },
    elkExample: elkProblem === null ? 'available' : `not available: ${elkProblem}`,
    summary,
    results,
  });

  console.log(`
flowchart geometry: ${results.length} fixtures`);
  console.log(`  within tolerance:    ${summary.within}`);
  console.log(`  outside tolerance:   ${summary.outside}`);
  console.log(
    `  not rendered:        ${summary.notRendered} (${[...notRenderedBy].map(([reason, n]) => `${n} x ${reason}`).join('; ') || 'none'})`
  );
  console.log(
    `  excluded:            ${summary.excluded} (${[...excludedBy].map(([reason, n]) => `${n} x ${reason}`).join('; ') || 'none'})`
  );
  console.log(`  reference failed:    ${summary.referenceFailed}`);
  console.log(`  foreignObject in reference: ${foreignObjectFixtures} fixtures (should be 0)`);
  if (elkProblem === null) {
    console.log(
      `  ELK input compared:  ${elkCompared.length} (${elkMatch} match, ${elkCompared.length - elkMatch} differ)`
    );
  }
  if (options.capture) console.log(`  captured under ${join(workDir, 'flowchart')}`);
  if (summary.outside > 0) process.exit(1);
}

try {
  await main();
} catch (error) {
  // Usage errors (an unknown stage, a listed fixture that is missing) print one line, not a stack.
  console.error((error as Error).message);
  process.exit(2);
}
