import assert from 'node:assert/strict';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '../..');
const files = [];
for (const crate of await readdir(path.join(root, 'crates'), { withFileTypes: true })) {
  if (!crate.isDirectory()) continue;
  for (const name of await readdir(path.join(root, 'crates', crate.name))) {
    if (name === 'NOTICE.md' || name.startsWith('LICENSE'))
      files.push(`crates/${crate.name}/${name}`);
  }
}
for (const name of await readdir(path.join(root, 'crates/ferrigura-text/fonts'))) {
  if (name.startsWith('LICENSE') || name.startsWith('OFL'))
    files.push(`crates/ferrigura-text/fonts/${name}`);
}
const sections = [
  '# Ferrigura native distribution notices\n\nThis distribution contains the Ferrigura engine, its layouts, and bundled fonts.\nThe following original notices and license texts accompany every native platform package.\n',
];
for (const name of files.sort())
  sections.push(`\n## ${name}\n\n${await readFile(path.join(root, name), 'utf8')}\n`);
const source = `${sections.join('\n')}\n`;
const destination = path.join(root, 'node/ferrigura/NOTICE.md');
if (process.argv.includes('--check'))
  assert.equal(
    await readFile(destination, 'utf8'),
    source,
    'Regenerate the native distribution notices'
  );
else await writeFile(destination, source);
