import { render, Renderer, type FontOptions, type RenderOptions } from '../index.mjs';
const options: RenderOptions = {
  id: 'example',
  colorScheme: 'auto',
  compat: true,
};
const svg: string = new Renderer({ theme: 'dark' }).render('pie\n"A" : 1', options);
const other: string = render('pie\n"B" : 2', {}, options);
void [svg, other];
render('flowchart LR\n A --> B', {}, { animation: {} });
render('flowchart LR\n A --> B', {}, { animation: { speed: 1.5, order: 'flow' } });
// @ts-expect-error Unsupported animation orders must be rejected statically.
render('flowchart LR\n A --> B', {}, { animation: { order: 'typo' } });
// @ts-expect-error Unsupported color schemes must be rejected statically.
render('pie', {}, { colorScheme: 'wrong' });
// @ts-expect-error The removed flowchart-only option must not reappear.
render('flowchart TD\n A --> B', {}, { flowchartMode: 'mermaid' });

// @ts-expect-error Compatibility is a boolean render option.
render('sequenceDiagram', {}, { compat: 'true' });

const fonts: FontOptions = {
  fonts: [{ family: 'Project Sans', data: new Uint8Array(), weight: 600, style: 'italic' }],
  fallbacks: ['Symbols'],
};
new Renderer({}, fonts);
render('flowchart LR\n A --> B', {}, {}, fonts);
new Renderer({}, { fonts: [{ family: 'Project Sans', path: '/explicit/font.ttf' }] });
// @ts-expect-error A face must select exactly one source.
new Renderer({}, { fonts: [{ family: 'Missing' }] });
// @ts-expect-error Bytes and paths cannot be supplied together.
new Renderer({}, { fonts: [{ family: 'Ambiguous', data: new Uint8Array(), path: '/font.ttf' }] });
// @ts-expect-error Only normal and italic styles are supported.
new Renderer({}, { fonts: [{ family: 'Style', data: new Uint8Array(), style: 'oblique' }] });
