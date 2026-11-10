import { readFileSync } from 'node:fs';
import { createNativeLoader } from './native-runtime.mjs';
const load = createNativeLoader(import.meta.url, {
  name: 'ferrigura',
  packagePattern: '@ferrigura/{target}',
  binaryPattern: 'ferrigura.node',
});

/** Render one diagram synchronously, or reuse Renderer for a configuration. */
export function render(text, config, options, fonts) {
  return load().render(text, config, options, fontInputs(fonts));
}
export class Renderer {
  #native;
  constructor(config, fonts) {
    this.#native = new (load().Renderer)(config, fontInputs(fonts));
  }
  render(text, options) {
    return this.#native.render(text, options);
  }
}

/** Files are a host concern. The addon receives owned bytes and explicit family names only. */
function fontInputs(options) {
  if (options == null || options.fonts == null) return options;
  return {
    ...options,
    fonts: options.fonts.map(({ path, data, ...face }) => {
      if ((path != null) === (data != null)) {
        throw new TypeError(
          `font ${JSON.stringify(face.family)} needs exactly one of path or data`
        );
      }
      return { ...face, data: path == null ? data : readFileSync(path) };
    }),
  };
}
