/**
 * Canonical JSON of quadrant chart diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The parser calls the DB in document order. The
 * DB's texts, points and classes are the state of the builder, which the DB keeps in a module
 * constant without an accessor, so the exporter reads it through `exposeBuilder`, a line the loader
 * appends to `quadrantDb.ts` (`openReference`'s extra plugins). The reference's source is not
 * changed. The model is that state before the builder lays it out (`docs-port/quadrant/PLAN.md`,
 * section 7).
 *
 * The DB's texts are what it holds: the placeholders of the entities stay. A point's coordinates
 * are the text the grammar read, which the DB converts with `+`, so the exporter writes `Number`
 * of it, and a value that is not finite as its sentinel string, as `value.rs` writes it.
 *
 * Usage: `tsx tools/port/export-quadrantChart.ts inputs.json outputs.json`
 */
import type { Plugin } from 'vite';

import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** The styles of a point or a class, with only the keys that were set. */
type Styles = Partial<{
  radius: number;
  color: string;
  strokeColor: string;
  strokeWidth: string;
}>;

interface BuilderPoint extends Styles {
  x: string | number;
  y: string | number;
  text: string;
  className: string;
}

interface QuadrantBuilder {
  data: {
    quadrant1Text: string;
    quadrant2Text: string;
    quadrant3Text: string;
    quadrant4Text: string;
    xAxisLeftText: string;
    xAxisRightText: string;
    yAxisBottomText: string;
    yAxisTopText: string;
    points: BuilderPoint[];
  };
  classes: Map<string, Styles>;
}

interface QuadrantDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
}

interface DbModule {
  default: QuadrantDb;
  /** The builder the DB writes to, which the appended line of `exposeBuilder` returns. */
  __exposeBuilder(): QuadrantBuilder;
}

interface Modules {
  config: {
    setSiteConfig(config: Record<string, unknown>): unknown;
    reset(): void;
    getConfig(): Record<string, unknown>;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Record<string, unknown>): void;
  };
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string };
      title?: string;
      config: Record<string, unknown>;
    };
  };
  detectType: {
    detectType(text: string, config?: unknown): string;
  };
  utils: {
    encodeEntities(text: string): string;
  };
  /** The Jison parser of `quadrant.jison`, whose `yy` is the DB. */
  grammar: { yy: unknown; parse(text: string): unknown };
  db: DbModule;
}

/** Appends the accessor of the builder to `quadrantDb.ts`, after its own code is compiled. */
const exposeBuilder: Plugin = {
  name: 'expose-quadrant-builder',
  enforce: 'post',
  transform(code, id) {
    if (!id.endsWith('/diagrams/quadrant-chart/quadrantDb.ts')) {
      return null;
    }
    return { code: `${code}\nexport const __exposeBuilder = () => quadrantBuilder;\n`, map: null };
  },
};

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, preprocess, detectType, utils, grammar, db] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/quadrant-chart/parser/quadrant.jison'
    ),
    reference.load<DbModule>('/packages/mermaid/src/diagrams/quadrant-chart/quadrantDb.ts'),
  ]);
  return { config, preprocess, detectType, utils, grammar: grammar.parser, db };
}

/** A coordinate as JSON: the number when it is finite, else `"NaN"`, `"Infinity"` or `"-Infinity"`. */
const coordinate = (value: number): number | string =>
  Number.isFinite(value) ? value : String(value);

/** The styles that were set, in a fixed order; a key that was not set is absent. */
function pickStyles(source: Styles): Styles {
  const styles: Styles = {};
  for (const key of ['radius', 'color', 'strokeColor', 'strokeWidth'] as const) {
    if (Object.hasOwn(source, key)) {
      Object.assign(styles, { [key]: source[key] });
    }
  }
  return styles;
}

await runExporter(
  loadModules,
  (input, { config, preprocess, detectType, utils, grammar, db }) => {
    // The configuration of the input alone, as a render gives it: the site settings, the reset,
    // the detected scope, then the front matter and directives (`processAndSetConfigs`). The
    // sanitising of the texts reads the security level and the HTML labels from it.
    config.setSiteConfig({});
    config.reset();
    const processed = preprocess.preprocessDiagram(input.text);
    let diagramType: string | undefined;
    try {
      diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
    } catch {
      diagramType = undefined;
    }
    config.setDiagramConfigScope(diagramType);
    config.addDirective(processed.config);

    const { code, title } = processed;
    const quadrantDb = db.default;
    quadrantDb.clear();
    grammar.yy = quadrantDb;
    // `Diagram.fromText` sets the front matter title before the parse.
    if (title) {
      quadrantDb.setDiagramTitle(title);
    }
    grammar.parse(utils.encodeEntities(code.cleaned) + '\n');

    const builder = db.__exposeBuilder();
    const { data, classes } = builder;
    return {
      version: 1,
      title: quadrantDb.getDiagramTitle(),
      accTitle: quadrantDb.getAccTitle(),
      accDescr: quadrantDb.getAccDescription(),
      quadrant1Text: data.quadrant1Text,
      quadrant2Text: data.quadrant2Text,
      quadrant3Text: data.quadrant3Text,
      quadrant4Text: data.quadrant4Text,
      xAxisLeftText: data.xAxisLeftText,
      xAxisRightText: data.xAxisRightText,
      yAxisBottomText: data.yAxisBottomText,
      yAxisTopText: data.yAxisTopText,
      points: data.points.map((point) => ({
        text: point.text,
        className: point.className,
        x: coordinate(Number(point.x)),
        y: coordinate(Number(point.y)),
        ...pickStyles(point),
      })),
      classes: [...classes].map(([name, styles]) => [name, pickStyles(styles)]),
    };
  },
  [exposeBuilder]
);
