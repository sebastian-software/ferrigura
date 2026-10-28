/**
 * Canonical values of YAML inputs, from `js-yaml` with the `JSON_SCHEMA` that the reference reads
 * the front matter, the flowchart shape data and the other YAML with.
 *
 * The canonical form is JSON with three markers, the same as `crates/ferrigura-syntax/examples/yaml.rs`
 * writes: `{"$undefined": true}` for `undefined` (an empty stream), and `{"$number": "NaN"}`,
 * `"Infinity"` or `"-Infinity"` for the non-finite numbers, which JSON cannot hold.
 *
 * Usage: `tsx tools/port/export-yaml.ts inputs.json outputs.json`
 */
import { readFile, writeFile } from 'node:fs/promises';

import { JSON_SCHEMA, load } from 'js-yaml';

interface YamlInput {
  id: number;
  text: string;
}

function canonical(value: unknown): unknown {
  if (value === undefined) return { $undefined: true };
  if (value === null || typeof value === 'boolean' || typeof value === 'string') return value;
  if (typeof value === 'number') {
    if (Number.isFinite(value)) return value;
    const name = Number.isNaN(value) ? 'NaN' : value > 0 ? 'Infinity' : '-Infinity';
    return { $number: name };
  }
  if (Array.isArray(value)) return value.map(canonical);
  if (typeof value === 'object') {
    // `defineProperty` keeps a key such as `__proto__` as an own property.
    const out: Record<string, unknown> = {};
    for (const key of Object.keys(value)) {
      Object.defineProperty(out, key, {
        value: canonical((value as Record<string, unknown>)[key]),
        enumerable: true,
        writable: true,
        configurable: true,
      });
    }
    return out;
  }
  throw new Error(`no canonical form for a ${typeof value}`);
}

const [inputPath, outputPath] = process.argv.slice(2);
if (!inputPath || !outputPath) {
  throw new Error('usage: tsx tools/port/export-yaml.ts <inputs.json> <outputs.json>');
}
const inputs = JSON.parse(await readFile(inputPath, 'utf8')) as YamlInput[];

const results = inputs.map((input) => {
  try {
    const value = load(input.text, { schema: JSON_SCHEMA });
    return { id: input.id, ok: true, value: canonical(value) };
  } catch (error) {
    return {
      id: input.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
});
await writeFile(outputPath, JSON.stringify(results, null, 2));
