/**
 * The scope of the colours of ferrigura's pie SVG in Chromium.
 *
 * Three pie diagrams are inlined in one page, each with its own id, and the page has a stylesheet of
 * its own that paints every path, text and rectangle. Each diagram must keep its own colours: the
 * computed colours of its title, its first slice, that slice's stroke and its first legend entry must
 * equal the values its own style declares. This is checked under the light and the dark colour scheme
 * the viewer may ask for, and with the style sheets off, where the presentation attributes must draw
 * the colours of the light set: the fallback for viewers without custom properties.
 *
 * This is a check of the cascade, not of the geometry; `pie-geometry.ts` compares the geometry.
 *
 * Usage: `pnpm port:scope-pie`
 */
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium, type Page } from 'playwright';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const workDir = join(root, 'target/scope');

const PIE = 'pie title Pets adopted\n  "Dogs" : 386\n  "Cats" : 85\n  "Rats" : 15\n';

/** The page's own rules. They would paint every path, text and rectangle if the diagrams did not scope theirs. */
const HOST_CSS = 'path, text, rect { fill: rebeccapurple; stroke: lime; }';

interface Diagram {
  id: string;
  scheme: 'light' | 'dark' | 'auto';
  themeVariables?: Record<string, string>;
}

const diagrams: Diagram[] = [
  { id: 'pie-auto', scheme: 'auto' },
  { id: 'pie-dark', scheme: 'dark' },
  { id: 'pie-custom', scheme: 'light', themeVariables: { pie1: '#00ff00' } },
];

/** The colours that are checked: the title, the first slice and its stroke, the first legend entry. */
interface Colours {
  title: string;
  slice: string;
  stroke: string;
  legend: string;
}

/** The colours as the page computes them, with the diagram's id. */
interface Measured extends Colours {
  id: string;
}

/** The custom property declarations of a diagram, by name. */
type Declared = Record<string, string>;

/** The page: measures the computed colours, and resolves declared values to the form the page computes. */
const HARNESS = `<!doctype html>
<html><head><meta charset="utf-8"><style>${HOST_CSS}</style></head>
<body><div id="stage"></div>
<script>
window.load = (renders) => {
  document.getElementById('stage').innerHTML = renders.map((svg) => '<div>' + svg + '</div>').join('');
};
const resolve = (value) => {
  const probe = document.createElement('span');
  probe.style.color = value;
  document.body.append(probe);
  const rgb = getComputedStyle(probe).color;
  probe.remove();
  return rgb;
};
window.resolve = (list) => list.map((colours) => ({
  title: resolve(colours.title),
  slice: resolve(colours.slice),
  stroke: resolve(colours.stroke),
  legend: resolve(colours.legend),
}));
window.measure = (ids) => ids.map((id) => {
  const root = document.getElementById(id);
  const slice = root.querySelector('path.pieCircle.pie-slot-0');
  return {
    id,
    title: getComputedStyle(root.querySelector('text.pieTitleText')).fill,
    slice: getComputedStyle(slice).fill,
    stroke: getComputedStyle(slice).stroke,
    legend: getComputedStyle(root.querySelector('g.legend rect.pie-slot-0')).fill,
  };
});
window.withoutStyles = (off) => {
  for (const sheet of document.styleSheets) sheet.disabled = off;
};
</script></body></html>`;

/** The declarations of a diagram's own rule: the top-level set, or the set under the dark media query. */
function declared(svg: string, id: string, dark: boolean): Declared {
  const style = /<style>([\S\s]*?)<\/style>/.exec(svg)?.[1] ?? '';
  const pattern = dark
    ? new RegExp(`@media \\(prefers-color-scheme: dark\\)\\{#${id}\\{([^}]*)\\}`)
    : new RegExp(`(?:^|\\})#${id}\\{([^}]*)\\}`);
  const body = pattern.exec(style)?.[1];
  if (body === undefined) {
    throw new Error(`${id} has no ${dark ? 'dark' : 'top-level'} declarations`);
  }
  const declarations: Declared = {};
  for (const declaration of body.split(';')) {
    const colon = declaration.indexOf(':');
    if (colon > 0)
      declarations[declaration.slice(0, colon).trim()] = declaration.slice(colon + 1).trim();
  }
  return declarations;
}

/** The colours a diagram declares for one set. */
function colours(declarations: Declared): Colours {
  return {
    title: need(declarations, '--pieTitleTextColor'),
    slice: need(declarations, '--pie1'),
    stroke: need(declarations, '--pieStrokeColor'),
    legend: need(declarations, '--pie1'),
  };
}

function need(declarations: Declared, name: string): string {
  const value = declarations[name];
  if (value === undefined) throw new Error(`the declarations have no ${name}`);
  return value;
}

async function measure(page: Page, ids: string[]): Promise<Measured[]> {
  return page.evaluate(
    (list) => (window as unknown as { measure: (ids: string[]) => Measured[] }).measure(list),
    ids
  );
}

async function resolveAll(page: Page, values: Colours[]): Promise<Colours[]> {
  return page.evaluate(
    (list) => (window as unknown as { resolve: (list: Colours[]) => Colours[] }).resolve(list),
    values
  );
}

/** Compares the measured colours of each diagram with its declared ones, and reports the differences. */
async function compare(
  page: Page,
  label: string,
  measured: Measured[],
  expected: Colours[],
  failures: string[]
): Promise<void> {
  const resolved = await resolveAll(page, expected);
  diagrams.forEach((diagram, index) => {
    const got = measured[index];
    const want = resolved[index];
    if (got?.id !== diagram.id || want === undefined) {
      failures.push(`${diagram.id} ${label}: not measured`);
      return;
    }
    const mismatches = (['title', 'slice', 'stroke', 'legend'] as const).filter(
      (key) => got[key] !== want[key]
    );
    if (mismatches.length === 0) {
      console.log(`✓ ${diagram.id} ${label}: title, slice, stroke and legend match`);
      return;
    }
    for (const key of mismatches) {
      failures.push(`${diagram.id} ${label}, ${key}: measured ${got[key]}, declared ${want[key]}`);
    }
  });
}

async function main(): Promise<void> {
  execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'render'], {
    cwd: root,
    stdio: 'inherit',
  });
  await mkdir(workDir, { recursive: true });

  const renders = new Map<string, string>();
  for (const diagram of diagrams) {
    const args = ['--id', diagram.id, '--scheme', diagram.scheme];
    if (diagram.themeVariables) {
      const configPath = join(workDir, `${diagram.id}.json`);
      await writeFile(configPath, JSON.stringify({ themeVariables: diagram.themeVariables }));
      args.push('--config', configPath);
    }
    const svg = execFileSync(join(root, 'target/debug/examples/render'), args, {
      input: PIE,
      cwd: root,
      maxBuffer: 16 * 1024 * 1024,
    }).toString('utf8');
    renders.set(diagram.id, svg);
  }
  const svgOf = (id: string): string => {
    const svg = renders.get(id);
    if (svg === undefined) throw new Error(`no SVG for ${id}`);
    return svg;
  };

  const ids = diagrams.map((diagram) => diagram.id);
  const failures: string[] = [];
  const browser = await chromium.launch({
    // Without hinting, Chromium on Linux keeps fractional glyph advances, as it does on macOS;
    // with the default, it rounds them to whole pixels and text measures differently.
    args: ['--font-render-hinting=none'],
  });
  try {
    const page = await browser.newPage();
    page.on('pageerror', (error) => failures.push(`page error: ${error.message}`));
    await page.setContent(HARNESS);
    await page.evaluate(
      (list) => (window as unknown as { load: (renders: string[]) => void }).load(list),
      [...renders.values()]
    );

    // Under each scheme the viewer asks for: an auto diagram follows it, the others keep their own set.
    for (const scheme of ['light', 'dark'] as const) {
      await page.emulateMedia({ colorScheme: scheme });
      const measured = await measure(page, ids);
      const expected = diagrams.map((diagram) =>
        colours(
          declared(svgOf(diagram.id), diagram.id, diagram.scheme === 'auto' && scheme === 'dark')
        )
      );
      await compare(page, `under ${scheme}`, measured, expected, failures);
    }

    // Without the style sheets, the presentation attributes draw: the light set of every diagram.
    await page.emulateMedia({ colorScheme: 'light' });
    await page.evaluate(() =>
      (window as unknown as { withoutStyles: (off: boolean) => void }).withoutStyles(true)
    );
    const attributes = await measure(page, ids);
    const fallback = diagrams.map((diagram) =>
      colours(declared(svgOf(diagram.id), diagram.id, false))
    );
    await compare(page, 'with the style sheets off', attributes, fallback, failures);
  } finally {
    await browser.close();
  }

  if (failures.length > 0) {
    for (const failure of failures) console.error(`✗ ${failure}`);
    process.exit(1);
  }
  console.log(`\nscope: ${diagrams.length} diagrams in one page, all colours as declared`);
}

await main();
