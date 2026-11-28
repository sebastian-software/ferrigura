import { execFileSync } from 'node:child_process';
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  truncateSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
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

void test('the default draws roles for every graph and compat draws Mermaid geometry', () => {
  const renderer = new Renderer();
  const flow = 'flowchart TD\n A([Start]) --> B{Review}\n B --> C[(Store)]\n';
  const drawn = renderer.render(flow);
  assert.match(drawn, /data-role="terminal"/);
  assert.match(drawn, /data-role="decision"/);
  assert.match(drawn, /data-role="data"/);
  assert.equal(renderer.render(flow, { compat: false }), drawn);
  assert.doesNotMatch(renderer.render(flow, { compat: true }), /data-role/);
  const states =
    'stateDiagram-v2\n [*] --> Open\n state pick <<choice>>\n Open --> pick\n pick --> [*]\n';
  assert.match(renderer.render(states), /data-role="decision"/);
  assert.doesNotMatch(renderer.render(states, { compat: true }), /data-role/);
  const classes = 'classDiagram\n class Shape {\n  <<interface>>\n }\n Shape <|.. Circle\n';
  assert.match(renderer.render(classes), /data-role="interface"/);
  assert.doesNotMatch(renderer.render(classes, { compat: true }), /data-role/);
  assert.match(renderer.render('pie\n "a" : 1\n'), /<svg/);
});

void test('compat selects Mermaid geometry for flowcharts and sequence participants', () => {
  const renderer = new Renderer();
  const sequence = 'sequenceDiagram\n participant App\n participant API\n App->>API: Request\n';
  const drawn = renderer.render(sequence);
  const compatible = renderer.render(sequence, { compat: true });
  assert.match(drawn, /height="44" name="App"/);
  assert.match(compatible, /height="76" name="App"/);
  assert.equal(renderer.render(sequence, { compat: false }), drawn);
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

test('host limits apply to text and flowchart edges', () => {
  const text = '%% 😀\npie\n"A": 1';
  for (const maxTextSize of [text.length, text.length + 1]) {
    assert.match(render(text, { maxTextSize }), /<svg/);
  }
  assert.throws(() => render(text, { maxTextSize: text.length - 1 }), /maxTextSize/);
  const flow = '---\nconfig:\n  maxEdges: 99\n---\nflowchart LR\n A --> B\n B --> C\n';
  for (const compat of [false, true]) {
    assert.match(render(flow, { maxEdges: 2, secure: [] }, { compat }), /<svg/);
    assert.match(render(flow, { maxEdges: 3 }, { compat }), /<svg/);
    assert.throws(() => render(flow, { maxEdges: 1, secure: [] }, { compat }), /Edge limit/);
  }
  assert.throws(() => render(text, { maxEdges: -1 }), /maxEdges/);
});

void test('fixed UTC milliseconds make Gantt output repeatable across entry points', () => {
  const text = 'gantt\n dateFormat HH:mm\n section Work\n Task :a, 00:00, 30d\n';
  const now = 1705276800000;
  const svg = render(text, {}, { now });
  assert.equal(render(text, {}, { now }), svg);
  assert.equal(new Renderer().render(text, { now }), svg);
  assert.notEqual(render(text, {}, { now: now / 1000 }), svg);
  for (const invalid of [NaN, Infinity, 1.5, 8640000000000001]) {
    assert.throws(() => render(text, {}, { now: invalid }), /now must be integer/);
  }
});

test('truncated and Unicode diagrams do not affect later renders', () => {
  const renderer = new Renderer();
  const control = 'pie\n"Cats": 2\n"Dogs": 3\n';
  const expected = renderer.render(control);
  const seeds = [
    'flowchart LR\nA[Grüße 🦀] --> B{Done}\n',
    'sequenceDiagram\nAlice->>Bob: こんにちは 🦀\n',
    'pie title Café\n"猫": 2\n"犬": 3\n',
    'stateDiagram-v2\n[*] --> Ready\nReady --> [*]\n',
    'classDiagram\nclass Animal\nAnimal : +String name\n',
  ];
  const cases = [
    '---\nconfig: [unterminated\n---\nflowchart LR\nA-->B',
    '---\nconfig:\n  flowchart:\n    padding: [wrong, type]\n---\nflowchart LR\nA-->B',
    'flowchart LR\nsubgraph outer\nsubgraph inner\nA -->',
    'sequenceDiagram\nAlice->>Bob: <&\0\n',
  ];
  const baselines = seeds.map((seed) => renderer.render(seed));
  for (const seed of seeds) {
    const characters = [...seed];
    for (let end = 0; end < characters.length; end += 7) {
      cases.push(characters.slice(0, end).join(''));
    }
    cases.push(seed);
  }
  for (const text of cases) {
    let svg;
    let succeeded = false;
    try {
      svg = renderer.render(text);
      succeeded = true;
    } catch (error) {
      assert.ok(error instanceof Error, `ordinary error for ${JSON.stringify(text)}`);
    }
    if (succeeded) assert.match(svg, /^<svg\b/);
    assert.equal(renderer.render(control), expected, `after ${JSON.stringify(text)}`);
    for (const [index, seed] of seeds.entries()) {
      assert.equal(renderer.render(seed), baselines[index], `after ${JSON.stringify(text)}`);
    }
  }
});

test('non-object configuration values are rejected while null and undefined use defaults', () => {
  const baseline = render(diagram, undefined, { id: 'cfg' });
  assert.equal(render(diagram, null, { id: 'cfg' }), baseline);
  assert.equal(new Renderer(null).render(diagram, { id: 'cfg' }), baseline);
  for (const invalid of [123, 'dark', true, false, [], [1, 2], Buffer.from('x')]) {
    assert.throws(() => render(diagram, invalid), /object|InvalidArg|expected/i);
    assert.throws(() => new Renderer(invalid), /object|InvalidArg|expected/i);
  }
});

test('fontInputs validates structure and guards local font files against non-regular or oversized files', () => {
  for (const badOptions of [123, 'bad', []]) {
    assert.throws(() => render(diagram, undefined, undefined, badOptions), TypeError);
    assert.throws(() => new Renderer(undefined, badOptions), TypeError);
  }
  for (const badFonts of ['not-an-array', 42, {}]) {
    assert.throws(() => render(diagram, undefined, undefined, { fonts: badFonts }), TypeError);
    assert.throws(() => new Renderer(undefined, { fonts: badFonts }), TypeError);
  }
  for (const badEntry of [null, undefined, 123, 'font', []]) {
    assert.throws(() => render(diagram, undefined, undefined, { fonts: [badEntry] }), TypeError);
    assert.throws(() => new Renderer(undefined, { fonts: [badEntry] }), TypeError);
  }
  assert.throws(
    () => new Renderer(undefined, { fonts: [{ family: 'BadData', data: 'not-bytes' }] }),
    /Uint8Array/
  );
  assert.throws(
    () =>
      new Renderer(undefined, {
        fonts: [{ family: 'TooBig', data: new Uint8Array(16 * 1024 * 1024 + 1) }],
      }),
    /exceeds size limit/
  );

  const scratch = mkdtempSync(join(tmpdir(), 'ferrigura-font-guard-'));
  try {
    // Rejects directories.
    assert.throws(
      () => new Renderer(undefined, { fonts: [{ family: 'Dir', path: scratch }] }),
      /not a regular file/
    );
    // Rejects oversized files (> 16 MiB).
    const huge = join(scratch, 'huge.ttf');
    writeFileSync(huge, '');
    truncateSync(huge, 16 * 1024 * 1024 + 1);
    assert.throws(
      () => new Renderer(undefined, { fonts: [{ family: 'Huge', path: huge }] }),
      /exceeds size limit/
    );
    // Rejects symlinks (where supported) and FIFOs (on Unix).
    const regular = join(scratch, 'real.ttf');
    writeFileSync(regular, fontData);
    const link = join(scratch, 'link.ttf');
    let symlinkCreated = false;
    try {
      symlinkSync(regular, link);
      symlinkCreated = true;
    } catch (error) {
      if (!(process.platform === 'win32' && ['EPERM', 'EACCES'].includes(error.code))) {
        throw error;
      }
    }
    if (symlinkCreated) {
      assert.throws(
        () => new Renderer(undefined, { fonts: [{ family: 'Link', path: link }] }),
        /not a regular file/
      );
      assert.throws(
        () => render(fontText, undefined, undefined, { fonts: [{ family: 'Link', path: link }] }),
        /not a regular file/
      );
    }
    if (process.platform !== 'win32') {
      const fifo = join(scratch, 'pipe.ttf');
      execFileSync('mkfifo', [fifo]);
      assert.throws(
        () => new Renderer(undefined, { fonts: [{ family: 'Fifo', path: fifo }] }),
        /not a regular file/
      );
    }
    // Deduplicated repeated path in one fonts array works cleanly.
    const dup = new Renderer(undefined, {
      fonts: [
        { family: 'Primary', path: regular },
        { family: 'Alias', path: regular },
      ],
    });
    assert.match(dup.render(fontText), /data:font\//);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test('repeated one-shot render calls with supplied fonts reuse cached state without cross-talk', () => {
  const sansPath = fileURLToPath(
    new URL('../../../crates/ferrigura-text/fonts/LexicaUltralegible-Regular.ttf', import.meta.url)
  );
  const sansData = readFileSync(sansPath);
  const options = { id: 'cached-fonts' };
  const monoFirst = render(fontText, undefined, options, {
    fonts: [{ family: 'Custom', data: fontData }],
  });
  const monoSecond = render(fontText, undefined, options, {
    fonts: [{ family: 'Custom', data: fontData }],
  });
  const sansSvg = render(fontText, undefined, options, {
    fonts: [{ family: 'Custom', data: sansData }],
  });
  assert.equal(monoFirst, monoSecond);
  assert.notEqual(monoFirst, sansSvg);
});
