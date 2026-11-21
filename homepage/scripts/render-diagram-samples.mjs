/* eslint-disable security/detect-non-literal-fs-filename -- Paths are a fresh temporary directory and fixed files in this repository. */
// Renders every homepage diagram from the current Rust source. The generated
// files are checked against the same source; callers use the single compat
// option for optimized and Mermaid-compatible drawings.
// Coordinates may differ by 0.01 on another architecture (published-output.mjs).
import assert from 'node:assert/strict';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { format } from 'oxfmt';

import { compareOutput } from './published-output.mjs';
import { createSourceRenderer } from './source-renderer.mjs';

const check = process.argv.includes('--check');
const provenance = 'Ferrigura (repository source)';
const target = new URL('../app/data/diagram-samples.json', import.meta.url);
const svgDirectory = new URL('../app/data/diagrams/', import.meta.url);
const recorded = check ? JSON.parse(await readFile(target, 'utf8')) : undefined;
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

const renderer = await createSourceRenderer();
try {
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
    adaptive: render('mode-adaptive', modeSource, { compat: false }),
    mermaid: render('mode-mermaid', modeSource, { compat: true }),
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
    renderedWith: { ferrigura: 'repository source' },
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
      `Verified ${svgs.size} homepage diagrams against ${provenance}${exact ? '' : ` (${platform}, numbers within 0.01 of ${recorded.platform})`}.`
    );
  } else {
    await rm(svgDirectory, { recursive: true, force: true });
    await mkdir(svgDirectory, { recursive: true });
    for (const [entry, content] of svgs) await writeFile(new URL(entry, svgDirectory), content);
    await writeFile(target, formatted.code);
    console.log(`Wrote ${svgs.size} homepage diagrams rendered by ${provenance}.`);
  }
} finally {
  await renderer.remove();
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

function compare({ actual, expected, label, exact }) {
  const message = `${label} drifted from ${provenance}; run pnpm sample:write`;
  compareOutput({ actual, expected, label, exact, message });
}
