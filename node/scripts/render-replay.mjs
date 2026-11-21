import { fileURLToPath } from 'node:url';
import { Renderer } from '../ferrigura/index.mjs';

const suffix = process.argv[2] ?? 'A';
const font = fileURLToPath(
  new URL('../../crates/ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf', import.meta.url)
);
import { cases } from './replay-cases.mjs';
const output = {};
for (const item of cases) {
  const fonts = item.font ? { fonts: [{ family: 'Project Mono', path: font }] } : undefined;
  const renderer = new Renderer(undefined, fonts);
  output[item.name] = renderer.render(item.text, {
    id: item.name + suffix,
    now: 1705276800000,
    compat: item.compat ?? false,
  });
}
process.stdout.write(JSON.stringify(output));
