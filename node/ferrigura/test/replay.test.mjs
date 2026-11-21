import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

// Each invocation starts a fresh process and renderer. Compare only runs of the same binary on
// the same target; cross-target equality is a separate question.
const worker = `
  import { Renderer } from ${JSON.stringify(new URL('../index.mjs', import.meta.url).href)};
  const font = process.argv[1];
  const suffix = process.argv[2];
  const flow = 'flowchart TD\\n A[Draft] --> B{Review}\\n B -->|Changes| A\\n B --> C[Publish]\\n';
  const cases = [
    { name: 'flow-default', text: flow },
    { name: 'flow-compat', text: flow, compat: true },
    { name: 'gantt', text: 'gantt\\n dateFormat HH:mm\\n section Work\\n Task :a, 00:00, 30d\\n' },
    { name: 'venn', text: 'venn-beta\\n set A\\n set B\\n set C\\n set D\\n union A, B\\n union C, D\\n union A, B, C\\n' },
    { name: 'architecture', text: '---\\nconfig:\\n  architecture:\\n    randomize: true\\n---\\narchitecture-beta\\n group g1(cloud)[Left]\\n group g2(cloud)[Right]\\n service a(server)[A] in g1\\n service b(database)[B] in g1\\n service c(disk)[C] in g2\\n service d(internet)[D]\\n a:R -- L:c\\n b:T -- B:a\\n c:B -- T:d\\n d:L -[Link]- R:b\\n' },
    { name: 'font', text: 'flowchart LR\\n A[Wide WWW narrow iii]\\n', font: true },
  ];
  const output = {};
  for (const item of cases) {
    const fonts = item.font ? { fonts: [{ family: 'Project Mono', path: font }] } : undefined;
    const renderer = new Renderer(undefined, fonts);
    output[item.name] = renderer.render(item.text, {
      id: item.name + suffix, now: 1705276800000, compat: item.compat ?? false,
    });
  }
  process.stdout.write(JSON.stringify(output));
`;
const fontPath = fileURLToPath(
  new URL(
    '../../../crates/ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf',
    import.meta.url
  )
);

function renderInNewProcess(suffix) {
  const child = spawnSync(
    process.execPath,
    ['--input-type=module', '-e', worker, fontPath, suffix],
    {
      encoding: 'utf8',
      timeout: 60_000,
      maxBuffer: 32 * 1024 * 1024,
    }
  );
  assert.ifError(child.error);
  assert.equal(child.status, 0, child.stderr);
  return JSON.parse(child.stdout);
}

function assertReplay(first, second) {
  for (const [name, svg] of Object.entries(first)) {
    assert.equal(second[name], svg, `${name} differs between processes`);
  }
}

test('representative diagrams replay byte for byte in independent processes', () => {
  const first = renderInNewProcess('A');
  const second = renderInNewProcess('A');
  assert.deepEqual(Object.keys(first), [
    'flow-default',
    'flow-compat',
    'gantt',
    'venn',
    'architecture',
    'font',
  ]);
  assert.deepEqual(Object.keys(second), Object.keys(first));
  for (const [name, svg] of Object.entries(first)) {
    assert.match(svg, /<svg/);
    assert.match(svg, new RegExp(`id="${name}A"`));
  }
  assertReplay(first, second);
  assert.match(first.font, /data:font\//);
  const differentId = renderInNewProcess('B');
  assert.throws(() => assertReplay(first, differentId), /flow-default differs between processes/);
});
