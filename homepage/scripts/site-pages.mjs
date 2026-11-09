/* eslint-disable security/detect-non-literal-fs-filename -- Discovery is limited to canonical repository Markdown. */
import { readdirSync, readFileSync } from 'node:fs';

const featured = new Map([
  ['homepage/content/getting-started.md', 'guide/getting-started'],
  ['docs/animation.md', 'guide/animation'],
  ['crates/ferrigura/README.md', 'reference/rust'],
  ['crates/ferrigura-cli/README.md', 'reference/cli'],
  ['crates/ferrigura-node/README.md', 'reference/node'],
  ['node/ferrigura/README.md', 'reference/npm'],
  ['README.md.src', 'reference/overview'],
  ['docs/diagrams/README.md', 'diagrams/overview'],
  ['docs/ARCHITECTURE.md', 'architecture/overview'],
  ['docs/DEVELOPING.md', 'guide/development'],
  ['docs/LEARNINGS.md', 'architecture/learnings'],
  ['docs/canonical-model.md', 'architecture/canonical-model'],
  ['docs/decisions/README.md', 'architecture/decisions'],
  ['docs/comparisons/README.md', 'comparisons/overview'],
  ['CHANGELOG.md', 'changelog'],
]);
const docs = readdirSync(new URL('../../docs/', import.meta.url), { recursive: true })
  .filter((file) => file.endsWith('.md'))
  .map((file) => `docs/${file.replaceAll('\\', '/')}`);
const titles = {
  'reference/rust': 'Rust library',
  'reference/cli': 'Command line',
  'reference/node': 'Node.js addon',
  'reference/npm': 'npm distribution',
  'reference/overview': 'Project overview',
  'diagrams/overview': 'Diagram types',
  'architecture/overview': 'Architecture',
  'architecture/decisions': 'Architecture decisions',
  'comparisons/overview': 'Choosing a renderer',
};
export const documents = [...new Set([...featured.keys(), ...docs])].map((source) => {
  const path =
    featured.get(source) ??
    source
      .replace(/^docs\/decisions\//, 'architecture/decisions/')
      .replace(/^docs\//, '')
      .replace(/\.md$/, '');
  const content = readFileSync(new URL(`../../${source}`, import.meta.url), 'utf8');
  const title = titles[path] ?? /^# (.+)$/m.exec(content)?.[1].replaceAll('`', '') ?? source;
  return { source, path, title, section: path.split('/')[0] };
});
/** Pages written in homepage/app/routes, each placed after a generated page. */
const authored = [
  {
    path: 'architecture/size',
    title: 'Binary size',
    section: 'architecture',
    after: 'architecture/overview',
  },
  {
    path: 'comparisons/side-by-side',
    title: 'Side by side with Mermaid',
    section: 'comparisons',
    after: 'comparisons/mermaid',
  },
];
/** Every documentation page in the order of the navigation. */
export const pages = documents.flatMap((document) => [
  document,
  ...authored.filter(({ after }) => after === document.path),
]);
if (pages.length !== documents.length + authored.length)
  throw new Error('An authored page follows no generated page.');
export const expectedPages = ['index.html', ...pages.map(({ path }) => `${path}/index.html`)];
export function pagePath(page) {
  return `/${page === 'index.html' ? '' : page.replace(/index\.html$/, '')}`;
}
