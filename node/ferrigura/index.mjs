import { closeSync, constants, fstatSync, lstatSync, openSync, readSync } from 'node:fs';
import { types } from 'node:util';
import { createNativeLoader } from './native-runtime.mjs';
const MAX_FONT = 16 * 1024 * 1024;
const load = createNativeLoader(import.meta.url, {
  name: 'ferrigura',
  packagePattern: '@ferrigura/{target}',
  binaryPattern: 'ferrigura.node',
});

/** Rejects non-object config values (including arrays and typed arrays/Buffers) before N-API. */
function validateConfig(config) {
  if (config === undefined || config === null) return config;
  if (
    typeof config !== 'object' ||
    Array.isArray(config) ||
    types.isArrayBufferView(config) ||
    types.isAnyArrayBuffer(config)
  ) {
    throw new TypeError('configuration must be an object');
  }
  return config;
}

/** Render one diagram synchronously, or reuse Renderer for a configuration. */
export function render(text, config, options, fonts) {
  return load().render(text, validateConfig(config), options, fontInputs(fonts));
}
export class Renderer {
  #native;
  constructor(config, fonts) {
    this.#native = new (load().Renderer)(validateConfig(config), fontInputs(fonts));
  }
  render(text, options) {
    return this.#native.render(text, options);
  }
}

/** Reads a regular font file up to MAX_FONT without following symlinks or blocking on FIFOs. */
function readFontFile(path) {
  const flags = constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0);
  let fd;
  try {
    if (!constants.O_NOFOLLOW) {
      const entry = lstatSync(path);
      if (!entry.isFile()) {
        throw new Error(`font file ${JSON.stringify(path)} is not a regular file`);
      }
    }
    fd = openSync(path, flags);
    const info = fstatSync(fd);
    if (!info.isFile()) {
      throw new Error(`font file ${JSON.stringify(path)} is not a regular file`);
    }
    if (info.size > MAX_FONT) {
      throw new Error(`font file ${JSON.stringify(path)} exceeds size limit`);
    }
    const buffer = Buffer.allocUnsafe(info.size + 1);
    let used = 0;
    while (used < buffer.length) {
      const bytesRead = readSync(fd, buffer, used, buffer.length - used, used);
      if (bytesRead === 0) break;
      used += bytesRead;
    }
    if (used > MAX_FONT) {
      throw new Error(`font file ${JSON.stringify(path)} exceeds size limit`);
    }
    if (used !== info.size) {
      throw new Error(`font file ${JSON.stringify(path)} changed size`);
    }
    return buffer.subarray(0, used);
  } catch (error) {
    if (error?.code === 'ELOOP') {
      throw new Error(`font file ${JSON.stringify(path)} is not a regular file`, {
        cause: error,
      });
    }
    throw error;
  } finally {
    if (fd !== undefined) closeSync(fd);
  }
}

/** Files are a host concern. The addon receives owned bytes and explicit family names only. */
function fontInputs(options) {
  if (options == null) return options;
  if (typeof options !== 'object' || Array.isArray(options)) {
    throw new TypeError('font options must be an object');
  }
  if (options.fonts == null) return options;
  if (!Array.isArray(options.fonts)) {
    throw new TypeError('fonts must be an array');
  }
  const byPath = new Map();
  return {
    ...options,
    fonts: options.fonts.map((entry) => {
      if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
        throw new TypeError('each font entry must be an object');
      }
      const { path, data, ...face } = entry;
      if ((path != null) === (data != null)) {
        throw new TypeError(
          `font ${JSON.stringify(face.family)} needs exactly one of path or data`
        );
      }
      if (path != null) {
        if (typeof path !== 'string') {
          throw new TypeError('font path must be a string');
        }
        let bytes = byPath.get(path);
        if (bytes === undefined) {
          bytes = readFontFile(path);
          byPath.set(path, bytes);
        }
        return { ...face, data: bytes };
      }
      if (!types.isUint8Array(data)) {
        throw new TypeError('font data must be a Uint8Array');
      }
      if (data.byteLength > MAX_FONT) {
        throw new RangeError(`font ${JSON.stringify(face.family)} exceeds size limit`);
      }
      return { ...face, data };
    }),
  };
}
