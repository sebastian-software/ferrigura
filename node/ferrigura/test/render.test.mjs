import { mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import test from 'node:test';
import { render, Renderer } from '../index.mjs';
const diagram = 'pie\n"A" : 1\n';
test('renders standalone and with a reusable renderer', () => {
  const renderer = new Renderer();
  assert.equal(
    renderer.render(diagram, { id: 'example' }),
    render(diagram, undefined, { id: 'example' })
  );
  assert.match(renderer.render(diagram, { id: 'second', colorScheme: 'auto' }), /<svg/);
  assert.match(renderer.render('flowchart LR\nA[Hello] --> B[World]', { id: 'flow' }), /Hello/);
});
test('errors propagate and leave a reused renderer usable', () => {
  const renderer = new Renderer();
  assert.throws(() => renderer.render('not a diagram'));
  assert.throws(() => renderer.render(diagram, { id: '0bad' }));
  assert.match(renderer.render(diagram, { id: 'after' }), /<svg/);
});

void test('compat selects Mermaid geometry for flowcharts and sequence participants', () => {
  const renderer = new Renderer();
  const sequence = 'sequenceDiagram\n participant App\n participant API\n App->>API: Request\n';
  const compact = renderer.render(sequence);
  const compatible = renderer.render(sequence, { compat: true });
  assert.match(compact, /height="32" name="App"/);
  assert.match(compatible, /height="76" name="App"/);
  assert.equal(renderer.render(sequence, { compat: false }), compact);
  assert.equal(renderer.render(sequence, { compat: true }), compatible);
  const flow = 'flowchart TD\n A[Draft] --> B{Review}\n B -->|Changes| A\n B --> C[Publish]\n';
  assert.notEqual(renderer.render(flow), renderer.render(flow, { compat: true }));
});

test('draw animation is optional, defaults to source order and validates its options', () => {
  const renderer = new Renderer();
  const text = 'flowchart LR\n A --> B\n B --> C\n';
  const plain = renderer.render(text, { id: 'drawing' });
  const animated = renderer.render(text, { id: 'drawing', animation: {} });
  assert.match(animated, /data-draw-duration=/);
  assert.match(animated, /prefers-reduced-motion:no-preference/);
  assert.equal(
    animated,
    render(text, undefined, { id: 'drawing', animation: { speed: 1, order: 'source' } })
  );
  assert.notEqual(animated, renderer.render(text, { id: 'drawing', animation: { speed: 2 } }));
  for (const speed of [0, -1, NaN, Infinity]) {
    assert.throws(() => renderer.render(text, { animation: { speed } }), /animation speed/);
  }
  assert.throws(() => renderer.render(text, { animation: { order: 'typo' } }), /animation order/);
  assert.throws(
    () => renderer.render('sequenceDiagram\n A->>B: Hi\n', { animation: {} }),
    /not supported for this diagram type/
  );
  assert.equal(plain, renderer.render(text, { id: 'drawing' }));
});

const fontPath = fileURLToPath(
  new URL(
    '../../../crates/ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf',
    import.meta.url
  )
);
const fontData = readFileSync(fontPath);
const fontText = 'flowchart LR\n A[Wide WWW narrow iii]\n';
test('font paths and bytes give the same owned, embedded custom-font output', () => {
  const options = { id: 'fonts' };
  const fromFile = new Renderer(undefined, { fonts: [{ family: 'Project Mono', path: fontPath }] });
  const fromBytes = { fonts: [{ family: 'Project Mono', data: new Uint8Array(fontData) }] };
  const custom = fromFile.render(fontText, options);
  assert.match(custom, /data:font\//);
  assert.notEqual(custom, render(fontText, undefined, options));
  assert.notEqual(
    custom.match(/viewBox="([^"]+)"/)[1],
    render(fontText, undefined, options).match(/viewBox="([^"]+)"/)[1]
  );
  assert.notEqual(
    custom.match(/data:font\/[^;]+;base64,([^"')]+)/)[1],
    render(fontText, undefined, options).match(/data:font\/[^;]+;base64,([^"')]+)/)[1]
  );
  assert.equal(custom, render(fontText, undefined, options, fromBytes));
  assert.equal(
    custom,
    render(fontText, undefined, options, {
      fonts: [{ family: 'Project Mono', path: fontPath, weight: 400, style: 'normal' }],
    })
  );
  assert.equal(render(fontText), render(fontText, undefined, undefined, {}));
});
test('facade rejects ambiguous sources and propagates local-file and registration errors', () => {
  for (const face of [{ family: 'Missing' }, { family: 'Both', path: fontPath, data: fontData }]) {
    assert.throws(() => new Renderer(undefined, { fonts: [face] }), /exactly one of path or data/);
    assert.throws(
      () => render(fontText, undefined, undefined, { fonts: [face] }),
      /exactly one of path or data/
    );
  }
  assert.throws(
    () => new Renderer(undefined, { fonts: [{ family: 'Weight', data: fontData, weight: 1001 }] }),
    /font weight/
  );
  assert.throws(
    () =>
      render(fontText, undefined, undefined, {
        fonts: [{ family: 'Style', data: fontData, style: 'oblique' }],
      }),
    /font style/
  );
  assert.throws(
    () => new Renderer(undefined, { fonts: [{ family: 'Invalid Path', path: 0 }] }),
    /font path must be a string/
  );
  assert.throws(
    () => new Renderer(undefined, { fonts: [{ family: 'Absent', path: `${fontPath}.missing` }] }),
    /ENOENT/
  );
  assert.throws(
    () =>
      new Renderer(undefined, { fonts: [{ family: 'Invalid', data: Buffer.from('not a font') }] }),
    /font/i
  );
});
test('an explicitly registered fallback renders an entire label', () => {
  const root = new URL('../../../crates/ferrigura-text/fonts/', import.meta.url);
  const fonts = {
    fonts: [
      { family: 'Primary', path: fileURLToPath(new URL('LexicaUltralegible-Regular.ttf', root)) },
      { family: 'Arrows', path: fileURLToPath(new URL('NotoSansMath-Regular-Arrows.ttf', root)) },
    ],
    fallbacks: ['Arrows'],
  };
  const renderer = new Renderer({ fontFamily: 'Primary' }, fonts);
  const text = 'flowchart LR\n A["→"]\n';
  const svg = renderer.render(text);
  assert.match(svg, /→/);
  assert.match(svg, /data:font\//);
  assert.equal(svg, render(text, { fontFamily: 'Primary' }, undefined, fonts));
});

test('a file is read once when its renderer is constructed', () => {
  const scratch = mkdtempSync(join(tmpdir(), 'ferrigura-font-'));
  try {
    const path = join(scratch, 'owned.ttf');
    writeFileSync(path, fontData);
    const renderer = new Renderer(undefined, { fonts: [{ family: 'Project Mono', path }] });
    unlinkSync(path);
    assert.equal(
      renderer.render(fontText),
      render(fontText, undefined, undefined, {
        fonts: [{ family: 'Project Mono', data: fontData }],
      })
    );
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test('empty host font options preserve bundled defaults', () => {
  const text = 'pie\n"Defaults" : 1\n';
  const baseline = render(text);
  for (const fonts of [{}, { fonts: [] }, { fallbacks: [] }, { fonts: [], fallbacks: [] }]) {
    assert.equal(render(text, undefined, undefined, fonts), baseline);
    assert.equal(new Renderer(undefined, fonts).render(text), baseline);
  }
});

test('diagram metadata selects registered names without loading its own font sources', () => {
  const fonts = { fonts: [{ family: 'Primary', data: fontData }] };
  for (const path of ['diagram-controlled-font.ttf', 'https://example.invalid/font.ttf']) {
    const directive = JSON.stringify({
      fontFamily: 'Injected',
      fonts: [{ family: 'Injected', path }],
    });
    const inputs = [
      `%%{init: ${directive}}%%\nflowchart LR\n A[Hello]\n`,
      `---\nconfig: ${directive}\n---\nflowchart LR\n A[Hello]\n`,
    ];
    for (const text of inputs) {
      assert.throws(
        () => render(text, undefined, undefined, fonts),
        (error) => {
          assert.match(error.message, /Injected/);
          assert.match(error.message, /register/);
          assert.doesNotMatch(error.message, /ENOENT|cannot read font/);
          return true;
        }
      );
    }
  }
});
