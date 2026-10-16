/**
 * Effective configuration from the TypeScript reference, for the differential run of
 * `ferrigura-model`'s configuration layers and `ferrigura-syntax`'s config reader.
 *
 * For each input this runs the steps `mermaidAPI` runs: the `initialize` settings become the site
 * config, the configuration is reset, the text goes through `preprocessDiagram`, its diagram type
 * is detected and the diagram scope is set to its section, and the merged front matter and
 * directive configuration is added. This is `processAndSetConfigs` of `mermaidAPI.ts`, which every
 * render runs. The output is the configuration the reference then returns from `getConfig()`, as
 * JSON, `themeVariables` included, with the section of the scope (`scope`, `null` when the type is
 * not detected). The key set the directives may use is written too.
 *
 * Usage: `tsx tools/port/export-config.ts inputs.json outputs.json`
 */
import { readFile, writeFile } from 'node:fs/promises';

import { openReference, registerDetectors, type Reference } from './reference.ts';

/** One input: the text, and the `initialize` settings it runs with, if any. */
interface ConfigInput {
  id: number;
  text: string;
  site?: Record<string, unknown>;
}

interface Modules {
  config: {
    setSiteConfig(conf: Record<string, unknown>): unknown;
    reset(): void;
    getConfig(): Record<string, unknown>;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Record<string, unknown>): void;
  };
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string };
      config: Record<string, unknown>;
    };
  };
  detectType: {
    detectType(text: string, config?: unknown): string;
  };
  defaultConfig: {
    configKeys: Set<string>;
  };
  diagramConfigKeys: {
    getDiagramConfigKey(diagramType: string): string;
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  await registerDetectors(reference);
  const [config, preprocess, detectType, defaultConfig, diagramConfigKeys] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['defaultConfig']>('/packages/mermaid/src/defaultConfig.ts'),
    reference.load<Modules['diagramConfigKeys']>(
      '/packages/mermaid/src/diagram-api/diagramConfigKeys.ts'
    ),
  ]);
  return { config, preprocess, detectType, defaultConfig, diagramConfigKeys };
}

/**
 * The effective configuration of one input, as the reference computes it, and the section of its
 * diagram scope, which is `null` when the type is not detected.
 */
function effectiveConfig(
  input: ConfigInput,
  modules: Modules
): { scope: string | null; config: Record<string, unknown> } {
  const { config, preprocess, detectType, diagramConfigKeys } = modules;

  config.setSiteConfig(input.site ?? {});
  config.reset();
  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
  } catch {
    diagramType = undefined;
  }
  config.setDiagramConfigScope(diagramType);
  config.addDirective(processed.config ?? {});

  const effective = jsonValue(config.getConfig()) as Record<string, unknown>;
  config.setDiagramConfigScope(undefined);
  const scope =
    diagramType === undefined ? null : diagramConfigKeys.getDiagramConfigKey(diagramType);
  return { scope, config: effective };
}

/**
 * The configuration as JSON, with the non-finite numbers written as the strings `"NaN"`,
 * `"Infinity"` and `"-Infinity"`, as `crates/ferrigura-syntax/examples/config.rs` writes them.
 * `JSON.stringify` would turn them into `null`, and the functions of the defaults away.
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

const [inputPath, outputPath] = process.argv.slice(2);
if (!inputPath || !outputPath) {
  throw new Error('usage: tsx tools/port/export-config.ts <inputs.json> <outputs.json>');
}
const inputs = JSON.parse(await readFile(inputPath, 'utf8')) as ConfigInput[];

const reference = await openReference();
try {
  const modules = await loadModules(reference);
  const results = inputs.map((input) => {
    try {
      return { id: input.id, ok: true, ...effectiveConfig(input, modules) };
    } catch (error) {
      return {
        id: input.id,
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  });
  // `$ref` is a leftover of the defaults generator in `.build/jsonSchema.ts` (an `xAxis` default
  // holds the raw reference), not a key anyone configures, so it is left out of the comparison.
  const configKeys = [...modules.defaultConfig.configKeys].filter((key) => key !== '$ref').sort();
  await writeFile(outputPath, JSON.stringify({ configKeys, results }, null, 2));
} finally {
  await reference.close();
}
