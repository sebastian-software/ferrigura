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
