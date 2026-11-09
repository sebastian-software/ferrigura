/* eslint-disable security/detect-non-literal-fs-filename -- Source and output paths come from the repository-owned document manifest. */
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { posix } from 'node:path';
import { format } from 'oxfmt';

import { documents, pages } from './site-pages.mjs';

const repository = new URL('../../', import.meta.url);
const routes = new URL('../app/routes/', import.meta.url);
const data = new URL('../app/data/', import.meta.url);
const destinations = new Map(documents.map(({ source, path }) => [source, `/${path}/`]));
destinations.set('README.md', '/reference/overview/');
const repoUrl = 'https://github.com/sebastian-software/ferrigura';
await mkdir(data, { recursive: true });
// The generated pages are Markdown; the pages written by hand beside them stay.
for (const section of ['guide', 'reference', 'diagrams', 'architecture', 'comparisons']) {
  const directory = new URL(`${section}/`, routes);
  const files = await readdir(directory, { recursive: true }).catch(() => []);
  for (const file of files.filter((name) => name.endsWith('.md')))
    await rm(new URL(file, directory));
}

for (const document of documents) {
  const source = await readFile(new URL(document.source, repository), 'utf8');
  const body = source.startsWith('---\n') ? source.slice(source.indexOf('\n---\n') + 5) : source;
  const content = rewriteLinks(
    body.replace(/^# .+\n/, '').replaceAll('```mermaid', '```text'),
    document.source
  );
  const destination = new URL(`${document.path}.md`, routes);
  await mkdir(new URL('.', destination), { recursive: true });
  await writeFile(destination, `---\ntitle: ${JSON.stringify(document.title)}\n---\n${content}`);
}

const cargo = await readFile(new URL('Cargo.toml', repository), 'utf8');
const version = /^version = "([^"]+)"/m.exec(cargo)?.[1];
if (!version) throw new Error('The package has no release version.');
const rustVersion = /^rust-version = "([^"]+)"/m.exec(cargo)?.[1];
if (!rustVersion) throw new Error('The workspace has no minimum Rust version.');
const npmManifest = JSON.parse(
  await readFile(new URL('node/ferrigura/package.json', repository), 'utf8')
);
const nodeVersion = /^>=\s*(\S+)$/.exec(npmManifest.engines?.node ?? '')?.[1];
if (!nodeVersion) throw new Error('The npm package has no minimum Node.js version.');
await writeData('site.json', { version, rustVersion, nodeVersion });
await writeData('coverage.json', await readCoverage());
const noticeDirectory = new URL('../public/notices/', import.meta.url);
await mkdir(noticeDirectory, { recursive: true });
const fontRoot = new URL('crates/ferrigura-text/', repository);
const noticeFiles = [
  'NOTICE.md',
  'fonts/OFL-Lexica.txt',
  'fonts/OFL-AtkinsonMono.txt',
  'fonts/OFL-NotoSansMath.txt',
  'fonts/OFL-NotoSansSymbols2.txt',
  'fonts/LICENSE-TwemojiMozilla.md',
];
const notices = await Promise.all(
  noticeFiles.map(async (name) => `${name}\n\n${await readFile(new URL(name, fontRoot), 'utf8')}`)
);
await writeFile(new URL('diagram-fonts.txt', noticeDirectory), notices.join('\n\n'));
// The face Mermaid's drawings on the side-by-side page name (homepage-specimens.css).
const fontDirectory = new URL('../public/fonts/', import.meta.url);
await mkdir(fontDirectory, { recursive: true });
for (const face of ['Regular', 'Bold', 'Italic', 'BoldItalic']) {
  const name = `LexicaUltralegible-${face}.ttf`;
  await writeFile(new URL(name, fontDirectory), await readFile(new URL(`fonts/${name}`, fontRoot)));
}
const labels = {
  guide: 'Guide',
  reference: 'Reference',
  diagrams: 'Diagrams',
  architecture: 'Architecture',
  comparisons: 'Comparisons',
  changelog: 'Changelog',
};
// Line icons from the family mark sprite, one per section in the section rail.
const icons = {
  guide: 'document',
  reference: 'package',
  diagrams: 'adapter',
  architecture: 'crate',
  comparisons: 'half',
  changelog: 'pen',
};
await writeData(
  'navigation.json',
  Object.entries(labels).map(([id, label]) => {
    const entries = pages
      .filter((page) => page.section === id)
      .map(({ path, title }) => ({ title, to: `/${path}/` }));
    return { id, label, icon: icons[id], to: entries[0].to, pages: entries };
  })
);
/**
 * The checks against Mermaid, per diagram type, from the coverage table of
 * docs/diagrams/README.md, which `pnpm -C tools/port port:status` rewrites.
 */
async function readCoverage() {
  const source = await readFile(new URL('docs/diagrams/README.md', repository), 'utf8');
  const [, mermaidTypes, mermaid] =
    /renders all (\d+) diagram types of Mermaid (\d+\.\d+\.\d+)/.exec(source) ?? [];
  if (!mermaid) throw new Error('docs/diagrams/README.md names no Mermaid version.');
  const types = source
    .split('\n')
    .filter((line) => /^\| \[[a-z0-9-]+\]\(\.\/[a-z0-9-]+\.md\)/.test(line))
    .map((line) => {
      const cells = line
        .split('|')
        .slice(1, -1)
        .map((cell) => cell.trim());
      const type = /^\[([^\]]+)\]/.exec(cells[0])?.[1];
      const parse = /^(\d+) inputs, no difference$/.exec(cells[4]);
      const geometry = /^(\d+) of (\d+) within tolerance/.exec(cells[6]);
      const left = /\((\d+) not compared\)$/.exec(cells[6]);
      if (!type || !parse || !geometry)
        throw new Error(`docs/diagrams/README.md: cannot read the coverage row ${cells[0]}`);
      return {
        type,
        layout: cells[2],
        parsed: Number(parse[1]),
        agreed: Number(geometry[1]),
        compared: Number(geometry[2]),
        notCompared: Number(left?.[1] ?? 0),
      };
    });
  if (types.length === 0) throw new Error('docs/diagrams/README.md has no coverage rows.');
  const sum = (key) => types.reduce((total, row) => total + row[key], 0);
  if (types.length !== Number(mermaidTypes))
    throw new Error(`docs/diagrams/README.md lists ${types.length} of ${mermaidTypes} types.`);
  return {
    mermaid,
    mermaidTypes: Number(mermaidTypes),
    types,
    parsed: sum('parsed'),
    agreed: sum('agreed'),
    compared: sum('compared'),
    notCompared: sum('notCompared'),
  };
}

async function writeData(name, value) {
  const result = await format(name, JSON.stringify(value));
  if (result.errors.length) throw new Error(`Cannot format ${name}`);
  await writeFile(new URL(name, data), result.code);
}

function rewriteLinks(markdown, source) {
  let fence;
  return markdown
    .split('\n')
    .map((line) => {
      const delimiter = /^\s*(`{3,}|~{3,})/.exec(line);
      if (delimiter) {
        if (!fence)
          fence = {
            character: delimiter[1][0],
            length: delimiter[1].length,
            rust: line.slice(delimiter[0].length).trim() === 'rust',
          };
        else if (delimiter[1][0] === fence.character && delimiter[1].length >= fence.length)
          fence = undefined;
        return line;
      }
      if (fence) return fence.rust && /^#(?: |$)/.test(line) ? null : line;
      return line.replaceAll(/(?<=\]\()[^\s)]+/g, (href) => resolveLink(href, source));
    })
    .filter((line) => line !== null)
    .join('\n');
}

function resolveLink(href, source) {
  if (/^(?:[a-z][a-z0-9+.-]*:|\/|#)/i.test(href)) return href;
  const [file, fragment] = href.split('#');
  const target = posix.normalize(posix.join(posix.dirname(source), file));
  const destination = destinations.get(target);
  if (destination) {
    return `${destination}${fragment ? `#${fragment}` : ''}`;
  }
  const kind = file.endsWith('/') || !posix.extname(file) ? 'tree' : 'blob';
  return `${repoUrl}/${kind}/main/${target}${fragment ? `#${fragment}` : ''}`;
}
