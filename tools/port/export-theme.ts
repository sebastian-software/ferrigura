/**
 * Theme variables from the TypeScript reference, for the differential run of `ferrigura-model`'s
 * themes. For each input this calls `getThemeVariables(overrides)` of the named shipped theme and
 * writes the own properties of the theme it returns, as JSON. A `null` override is passed as no
 * overrides, the `undefined` the reference's `getThemeVariables` takes for them; the reference
 * itself throws on `null`, which mermaid never passes.
 *
 * Usage: `pnpm exec tsx tools/port/export-theme.ts inputs.json outputs.json`
 */
import { readFile, writeFile } from 'node:fs/promises';

import { openReference, type Reference } from './reference.ts';

/** One input: the theme by its configured name, and the overrides it runs with, if any. */
interface ThemeInput {
  id: number;
  theme: string;
  overrides: Record<string, unknown> | null;
}

/** The module of a theme: its `getThemeVariables`, which returns the computed theme. */
interface ThemeModule {
  getThemeVariables(userOverrides?: Record<string, unknown>): object;
}

const modulePaths: Record<string, string> = {
  'redux-color': '/packages/mermaid/src/themes/theme-redux-color.js',
  'redux-dark-color': '/packages/mermaid/src/themes/theme-redux-dark-color.js',
};

async function loadThemes(reference: Reference): Promise<Record<string, ThemeModule>> {
  const themes: Record<string, ThemeModule> = {};
  for (const [name, path] of Object.entries(modulePaths)) {
    themes[name] = await reference.load<ThemeModule>(path);
  }
  return themes;
}

/**
 * The value as JSON: non-finite numbers become the strings `"NaN"`, `"Infinity"` and `"-Infinity"`,
 * as `crates/ferrigura-model/examples/theme.rs` writes them, and `undefined` properties are left
 * out, as `JSON.stringify` leaves them out.
 */
function jsonValue(value: unknown): unknown {
  if (typeof value === 'number') {
    if (Number.isFinite(value)) return value;
    return Number.isNaN(value) ? 'NaN' : value > 0 ? 'Infinity' : '-Infinity';
  }
  if (typeof value === 'function' || value === undefined) return undefined;
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map((item) => jsonValue(item) ?? null);
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(value)) {
    const item = jsonValue((value as Record<string, unknown>)[key]);
    if (item !== undefined) {
      Object.defineProperty(out, key, {
        value: item,
        enumerable: true,
        writable: true,
        configurable: true,
      });
    }
  }
  return out;
}

/** The variables of one input, as the reference's theme holds them after the overrides. */
function variablesOf(input: ThemeInput, themes: Record<string, ThemeModule>): unknown {
  const theme = themes[input.theme];
  if (!theme) throw new Error(`unknown theme "${input.theme}"`);
  const instance = theme.getThemeVariables(input.overrides ?? undefined);
  return jsonValue(instance);
}

const [inputPath, outputPath] = process.argv.slice(2);
if (!inputPath || !outputPath) {
  throw new Error('usage: tsx tools/port/export-theme.ts <inputs.json> <outputs.json>');
}
const inputs = JSON.parse(await readFile(inputPath, 'utf8')) as ThemeInput[];

const reference = await openReference();
try {
  const themes = await loadThemes(reference);
  const results = inputs.map((input) => {
    try {
      return { id: input.id, ok: true, variables: variablesOf(input, themes) };
    } catch (error) {
      return {
        id: input.id,
        ok: false,
        variables: {},
        error: error instanceof Error ? error.message : String(error),
      };
    }
  });
  await writeFile(outputPath, JSON.stringify(results, null, 2));
} finally {
  await reference.close();
}
