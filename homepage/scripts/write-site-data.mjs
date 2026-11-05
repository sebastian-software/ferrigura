/* eslint-disable security/detect-non-literal-fs-filename -- Source and output paths come from the repository-owned document manifest. */
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { posix } from 'node:path';
import { format } from 'oxfmt';

import { documents } from './site-pages.mjs';

const repository = new URL('../../', import.meta.url);
const routes = new URL('../app/routes/', import.meta.url);
const data = new URL('../app/data/', import.meta.url);
const destinations = new Map(documents.map(({ source, path }) => [source, `/${path}/`]));
destinations.set('README.md', '/reference/overview/');
const repoUrl = 'https://github.com/sebastian-software/ferrigura';
await mkdir(data, { recursive: true });
for (const section of ['guide', 'reference', 'diagrams', 'architecture', 'comparisons']) {
  await rm(new URL(`${section}/`, routes), { recursive: true, force: true });
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
await writeData('site.json', { version });
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
    const pages = documents
      .filter((document) => document.section === id)
      .map(({ path, title }) => ({ title, to: `/${path}/` }));
    return { id, label, icon: icons[id], to: pages[0].to, pages };
  })
);
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
