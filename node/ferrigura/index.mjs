import { createNativeLoader } from './native-runtime.mjs';
const load = createNativeLoader(import.meta.url, {
  name: 'ferrigura',
  packagePattern: '@ferrigura/{target}',
  binaryPattern: 'ferrigura.node',
});

/** Render one diagram synchronously, or reuse Renderer for a configuration. */
export function render(text, config, options) {
  return load().render(text, config, options);
}
export class Renderer {
  #native;
  constructor(config) {
    this.#native = new (load().Renderer)(config);
  }
  render(text, options) {
    return this.#native.render(text, options);
  }
}
