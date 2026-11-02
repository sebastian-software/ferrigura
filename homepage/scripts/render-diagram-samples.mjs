/* eslint-disable security/detect-non-literal-fs-filename -- The sample paths and fresh temporary build are owned by this script. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { format } from 'oxfmt';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = new URL('../app/data/diagram-samples.json', import.meta.url);
const work = await mkdtemp(join(tmpdir(), 'ferrigura-homepage-'));
const samples = [
  {
    id: 'flowchart',
    label: 'Flowchart',
    description:
      'A draft goes to review. Approval leads to publication; requested changes return it to draft.',
    source:
      'flowchart TD\n  draft[Draft] --> review{Review}\n  review -->|Approved| publish[Publish]\n  review -->|Changes| draft\n',
  },
  {
    id: 'sequence',
    label: 'Sequence',
    description:
      'The editor asks the renderer to render Mermaid text. The renderer returns SVG to the editor.',
    source:
      'sequenceDiagram\n  participant E as Editor\n  participant R as Renderer\n  E->>R: Mermaid text\n  R-->>E: Standalone SVG\n',
  },
  {
    id: 'state',
    label: 'State',
    description: 'A document starts as draft, enters review, and becomes published after approval.',
    source:
      'stateDiagram-v2\n  [*] --> Draft\n  Draft --> Review: submit\n  Review --> Published: approve\n  Published --> [*]\n',
  },
];
try {
  for (const sample of samples) {
    const input = join(work, `${sample.id}.mmd`);
    await writeFile(input, sample.source);
    const render = () =>
      execFileSync(
        'cargo',
        [
          'run',
          '--quiet',
          '--locked',
          '-p',
          'ferrigura-cli',
          '--',
          '--id',
          `homepage-${sample.id}`,
          '--scheme',
          'light',
          input,
        ],
        { cwd: root, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 }
      );
    const svg = render();
    assert.equal(svg, render(), `${sample.id} must render deterministically`);
    assert.match(svg, /<text\b/);
    assert.match(svg, /@font-face/);
    assert.doesNotMatch(svg, /<foreignObject\b|<script\b/);
    sample.svg = svg;
    const publicDirectory = new URL('../public/examples/', import.meta.url);
    await mkdir(publicDirectory, { recursive: true });
    if (process.argv.includes('--check'))
      assert.equal(await readFile(new URL(`${sample.id}.svg`, publicDirectory), 'utf8'), svg);
    else await writeFile(new URL(`${sample.id}.svg`, publicDirectory), svg);
  }
  const formatted = await format('diagram-samples.json', JSON.stringify({ samples }));
  assert.equal(formatted.errors.length, 0);
  if (process.argv.includes('--check'))
    assert.equal(
      await readFile(output, 'utf8'),
      formatted.code,
      'Refresh the native SVG samples with pnpm sample:write.'
    );
  else await writeFile(output, formatted.code);
  console.log('Verified three repeatable SVG samples with the native Ferrigura CLI.');
} finally {
  await rm(work, { recursive: true, force: true });
}
