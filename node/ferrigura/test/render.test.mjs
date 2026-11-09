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
