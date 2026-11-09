import { render, Renderer, type RenderOptions } from '../index.mjs';
const options: RenderOptions = {
  id: 'example',
  colorScheme: 'auto',
  compat: true,
  flowchartMode: 'mermaid',
};
const svg: string = new Renderer({ theme: 'dark' }).render('pie\n"A" : 1', options);
const other: string = render('pie\n"B" : 2', {}, options);
void [svg, other];
// @ts-expect-error Unsupported color schemes must be rejected statically.
render('pie', {}, { colorScheme: 'wrong' });
// @ts-expect-error Unsupported flowchart policies must be rejected statically.
render('flowchart TD\n A --> B', {}, { flowchartMode: 'wrong' });

// @ts-expect-error Compatibility is a boolean render option.
render('sequenceDiagram', {}, { compat: 'true' });
