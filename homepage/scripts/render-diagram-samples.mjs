/* eslint-disable security/detect-non-literal-fs-filename -- Paths are a fresh temporary directory and fixed files in this repository. */
// Renders every diagram on the homepage with the published @ferrigura/core
// package and writes app/data/diagram-samples.json and the SVG files in
// app/data/diagrams/. The page imports them; nobody edits them by hand.
//
//   node scripts/render-diagram-samples.mjs          # write the files
//   node scripts/render-diagram-samples.mjs --check  # fail when they drifted
//
// Writing renders with the version the site documents (node/ferrigura/package.json)
// and records it in `renderedWith`. The check renders again with the recorded
// version, so a release pull request that bumps the manifest before npm has the
// new version still passes; the page keeps crediting the release that produced
// its diagrams until the next write. The package installs from npm into a
// temporary directory with lifecycle scripts disabled, so the site's own
// dependency tree stays untouched. Run the write again after a release.
//
// The output is the same on every run of one machine. On another architecture,
// a few trigonometric results can differ in their last digits (docs/LEARNINGS.md,
// "fused multiply-adds"), so the check compares numbers within 0.01 there.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { format } from 'oxfmt';

const check = process.argv.includes('--check');
const name = '@ferrigura/core';
const target = new URL('../app/data/diagram-samples.json', import.meta.url);
const svgDirectory = new URL('../app/data/diagrams/', import.meta.url);
const recorded = check ? JSON.parse(await readFile(target, 'utf8')) : undefined;
const manifest = JSON.parse(
  await readFile(new URL('../../node/ferrigura/package.json', import.meta.url), 'utf8')
);
const version = recorded?.renderedWith[name] ?? manifest.version;
const platform = `${process.platform}-${process.arch}`;

/** The gallery: one everyday diagram of six types, each with a plain description. */
const gallery = [
  {
    id: 'flowchart',
    label: 'Flowchart',
    description:
      'A request is checked for a signed-in visitor, then for a cached page; an uncached page is rendered first.',
    source: `flowchart TD
  request[Incoming request] --> auth{Signed in?}
  auth -->|yes| cache{In the cache?}
  auth -->|no| login[Sign-in page]
  cache -->|yes| page[Cached page]
  cache -->|no| render[Render the page]
  render --> page
`,
  },
  {
    id: 'sequence',
    label: 'Sequence',
    description:
      'A reader opens a guide. The site asks the build for the page, the build returns HTML with inline SVG, and the site sends it to the reader.',
    source: `sequenceDiagram
  actor Reader
  participant Site as Documentation site
  participant Build
  Reader->>Site: Open the guide
  Site->>Build: Page not cached
  Build-->>Site: HTML with inline SVG
  Site-->>Reader: The page, diagrams included
`,
  },
  {
    id: 'class',
    label: 'Class',
    description:
      'A document holds any number of blocks. Block is an interface that paragraphs and diagrams implement.',
    source: `classDiagram
  class Document {
    +String title
    +List~Block~ blocks
    +render() String
  }
  class Block {
    <<interface>>
    +toHtml() String
  }
  class Paragraph
  class Diagram {
    +String source
    +toSvg() String
  }
  Document "1" o-- "*" Block
  Block <|.. Paragraph
  Block <|.. Diagram
`,
  },
  {
    id: 'state',
    label: 'State',
    description:
      'A page starts as a draft, goes to review, returns to draft when changes are requested, and is published, then archived.',
    source: `stateDiagram-v2
  [*] --> Draft
  Draft --> Review: submit
  Review --> Draft: request changes
  Review --> Published: approve
  Published --> Archived: retire
  Archived --> [*]
`,
  },
  {
    id: 'er',
    label: 'Entity relationship',
    description:
      'An author writes articles, an article contains sections, and a section shows diagrams.',
    source: `erDiagram
  AUTHOR ||--o{ ARTICLE : writes
  ARTICLE ||--|{ SECTION : contains
  SECTION ||--o{ DIAGRAM : shows
  AUTHOR {
    string name
    string email
  }
  ARTICLE {
    string slug
    date published
  }
`,
  },
  {
    id: 'gitgraph',
    label: 'Git graph',
    description:
      'A docs branch leaves main, gains two commits and is merged back after a fix on main.',
    source: `gitGraph
  commit id: "Initial"
  branch docs
  checkout docs
  commit id: "Add guide"
  commit id: "Add diagrams"
  checkout main
  commit id: "Fix build"
  merge docs
  commit id: "Release"
`,
  },
];

/** The flowchart the two rendering modes are compared on: a review cycle with labels. */
const modeSource = `flowchart TD
  write[Write the guide] --> review{Review}
  review -->|Approved| publish[Publish]
  review -->|Changes requested| write
  publish --> announce[Announce the release]
`;
const modeDescription =
  'A guide is written and reviewed. Approval publishes it and the release is announced; requested changes send it back to writing.';

const directory = await mkdtemp(join(tmpdir(), 'ferrigura-homepage-samples-'));
try {
  await writeFile(
    join(directory, 'package.json'),
    `${JSON.stringify({ private: true, type: 'module', dependencies: { [name]: version } }, null, 2)}\n`
  );
  execFileSync('npm', ['install', '--ignore-scripts', '--no-audit', '--no-fund'], {
    cwd: directory,
    stdio: check ? 'ignore' : 'inherit',
  });
  const installed = JSON.parse(
    await readFile(join(directory, 'node_modules', name, 'package.json'), 'utf8')
  );
  assert.equal(installed.version, version, `Installed ${name} ${installed.version}`);
  const require = createRequire(join(directory, 'package.json'));
  const { Renderer } = await import(pathToFileURL(require.resolve(name)).href);
  const renderer = new Renderer();

  const svgs = new Map();
  /** Renders twice, checks the contract the page describes, and keeps the file. */
  const render = (file, source, options) => {
    const svg = renderer.render(source, {
      colorScheme: 'light',
      ...options,
      id: `homepage-${file}`,
    });
    assert.equal(
      svg,
      renderer.render(source, { colorScheme: 'light', ...options, id: `homepage-${file}` }),
      `${file} must render the same twice`
    );
    assert.match(svg, /<text\b/, `${file} has no SVG text`);
    assert.match(svg, /@font-face/, `${file} embeds no font`);
    assert.doesNotMatch(svg, /<foreignObject\b|<script\b/, `${file} contains HTML or script`);
    svgs.set(`${file}.svg`, svg);
    return `${file}.svg`;
  };

  const samples = gallery.map((sample) => ({ ...sample, file: render(sample.id, sample.source) }));
  const modes = {
    source: modeSource,
    description: modeDescription,
    adaptive: render('mode-adaptive', modeSource, { flowchartMode: 'adaptive' }),
    mermaid: render('mode-mermaid', modeSource, { flowchartMode: 'mermaid' }),
  };
  assert.notEqual(
    svgs.get(modes.adaptive),
    svgs.get(modes.mermaid),
    'The two flowchart modes must differ on the compared source'
  );
  const sequence = gallery.find(({ id }) => id === 'sequence');
  const schemes = {
    source: sequence.source,
    description: sequence.description,
    light: render('scheme-light', sequence.source, { colorScheme: 'light' }),
    dark: render('scheme-dark', sequence.source, { colorScheme: 'dark' }),
  };

  // What one file carries, counted in the first gallery diagram.
  const first = samples[0];
  const svg = svgs.get(first.file);
  const fonts = [...svg.matchAll(/url\(data:font\/[a-z0-9]+;base64,([A-Za-z0-9+/=]+)\)/g)];
  // The decision node's label: each word placed at the width the bundled font gives it.
  const node = svg.slice(svg.indexOf('-flowchart-auth-'));
  const label = /<g class="label"[^>]*>[\s\S]*?<\/text><\/g><\/g>/.exec(node)?.[0];
  assert(label, 'The flowchart has no label for its decision node');
  const file = {
    sample: first.id,
    bytes: Buffer.byteLength(svg),
    fontFaces: fonts.length,
    fontBytes: fonts.reduce((total, [, data]) => total + Buffer.from(data, 'base64').length, 0),
    textElements: svg.match(/<text\b/g).length,
    htmlElements: svg.match(/<foreignObject\b/g)?.length ?? 0,
    scripts: svg.match(/<script\b/g)?.length ?? 0,
    // One label as it stands in the file, indented at each tag for reading.
    label: indent(label),
  };
  assert(file.fontFaces > 0 && file.fontBytes > 0, 'The flowchart embeds no font data');

  const exact = !check || recorded.platform === platform;
  // Elsewhere the check compares against the recording; a digit more or less in
  // a coordinate may change the file's size by a few bytes.
  if (!exact) file.bytes = recorded.file.bytes;
  const data = {
    renderedWith: { [name]: version },
    platform: check ? recorded.platform : platform,
    samples,
    modes,
    schemes,
    file,
  };
  const formatted = await format('diagram-samples.json', JSON.stringify(data));
  assert.equal(formatted.errors.length, 0);

  if (check) {
    const json = await readFile(target, 'utf8');
    compare({ actual: json, expected: formatted.code, label: 'diagram-samples.json', exact });
    const entries = await readdir(svgDirectory);
    const committed = entries.filter((entry) => entry.endsWith('.svg'));
    assert.deepEqual(committed.toSorted(), [...svgs.keys()].toSorted(), 'The SVG files drifted');
    for (const [entry, content] of svgs) {
      const actual = await readFile(new URL(entry, svgDirectory), 'utf8');
      compare({ actual, expected: content, label: entry, exact });
    }
    console.log(
      `Verified ${svgs.size} homepage diagrams against ${name} ${version}${exact ? '' : ` (${platform}, numbers within 0.01 of ${recorded.platform})`}.`
    );
  } else {
    await rm(svgDirectory, { recursive: true, force: true });
    await mkdir(svgDirectory, { recursive: true });
    for (const [entry, content] of svgs) await writeFile(new URL(entry, svgDirectory), content);
    await writeFile(target, formatted.code);
    console.log(`Wrote ${svgs.size} homepage diagrams rendered by ${name} ${version}.`);
  }
} finally {
  await rm(directory, { recursive: true, force: true });
}

/** Breaks a fragment of SVG markup at its tags and indents it by depth. */
function indent(markup) {
  const lines = [];
  let depth = 0;
  let open = false;
  for (const piece of markup.split(/(?=<)/)) {
    if (piece.startsWith('</')) {
      depth--;
      // An element without child elements closes on its own line.
      if (open) lines[lines.length - 1] += piece;
      else lines.push(`${'  '.repeat(depth)}${piece}`);
      open = false;
      continue;
    }
    lines.push(`${'  '.repeat(depth)}${piece}`);
    open = !piece.endsWith('/>');
    if (open) depth++;
  }
  return lines.join('\n');
}

/** Exact on the recording platform; elsewhere every number may move by 0.01. */
function compare({ actual, expected, label, exact }) {
  const message = `${label} drifted from ${name} ${version}; run pnpm sample:write`;
  if (exact) {
    assert.equal(actual, expected, message);
    return;
  }
  const number = /-?\d[\d.]*/g;
  assert.equal(actual.replaceAll(number, '#'), expected.replaceAll(number, '#'), message);
  const left = actual.match(number) ?? [];
  const right = expected.match(number) ?? [];
  assert.equal(left.length, right.length, message);
  left.forEach((value, index) => {
    const other = right[index];
    // Version numbers and other dotted runs must match exactly.
    const near = Math.abs(Number(value) - Number(other)) <= 0.01;
    assert(value === other || near, `${message}: ${value} ≠ ${other}`);
  });
}
