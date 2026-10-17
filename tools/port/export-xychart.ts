/**
 * Canonical JSON of xychart diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram` (front matter, directives, comments), the configuration
 * that the text carries is set as `mermaidAPI` sets it, and then `Diagram.fromText` runs the
 * steps: the DB is cleared, the front matter title is set, and the Jison parser reads the text
 * with `encodeEntities` and a line break. The model is the DB state after the parse. The chart
 * builder does not run.
 *
 * The palette of a plot is the `plotColorPalette` of the theme the configuration resolves to. The
 * port ships the two redux themes only (M6 decision), and a theme that is not shipped resolves to
 * one of them, as `theme_name` and `Theme::from_name` resolve it; the exporter applies the same
 * rule to the reference's theme, so both sides draw the palette of the same theme (see
 * `docs-port/xychart/PLAN.md`, section 6).
 *
 * Usage: `tsx tools/port/export-xychart.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type Point = [string, number | undefined];

interface XyChartData {
  xAxis:
    | { type: 'band'; title: string; categories: string[] }
    | { type: 'linear'; title: string; min: number; max: number };
  yAxis: { type: 'linear'; title: string; min: number; max: number };
  title: string;
  plots: (
    | {
        type: 'line';
        title: string;
        strokeFill: string;
        strokeWidth: number;
        data: Point[];
        pointLabels?: string[];
      }
    | { type: 'bar'; title: string; fill: string; data: Point[] }
  )[];
}

interface XyChartDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getChartConfig(): { chartOrientation: string };
  getXYChartData(): XyChartData;
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
  /** The Jison parser of `xychart.jison`, whose `yy` is the DB. */
  grammar: { parser: { yy: unknown; parse(text: string): unknown } };
  db: { default: XyChartDb };
}

/** The themes ferrigura ships. Any other theme name, and no theme, resolves to one of them. */
const SHIPPED_THEMES = ['redux-color', 'redux-dark-color'];

/** The shipped theme a configured theme resolves to: names ending in `dark` take the dark one. */
function shippedTheme(theme: unknown): string {
  if (typeof theme === 'string' && SHIPPED_THEMES.includes(theme)) return theme;
  return typeof theme === 'string' && theme.endsWith('dark') ? 'redux-dark-color' : 'redux-color';
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, preprocess, detectType, utils, grammar, db] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar']['parser'] }>(
      '/packages/mermaid/src/diagrams/xychart/parser/xychart.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/xychart/xychartDb.ts'),
  ]);
  return { config, preprocess, detectType, utils, grammar: grammar, db };
}

/** A score or an axis bound as JSON: the number when it is finite, else its sentinel string. */
const number = (value: number): number | string => (Number.isFinite(value) ? value : String(value));

await runExporter(loadModules, (input, { config, preprocess, detectType, utils, grammar, db }) => {
  // The configuration of the input alone, as a render gives it: the site settings, the reset,
  // the detected scope, then the front matter and directives (`processAndSetConfigs`).
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
  processed.config.theme = shippedTheme(processed.config.theme);
  config.addDirective(processed.config);

  const xyChartDb = db.default;
  xyChartDb.clear();
  grammar.parser.yy = xyChartDb;
  // `Diagram.fromText` sets the front matter title before the parse.
  if (processed.title) {
    xyChartDb.setDiagramTitle(processed.title);
  }
  grammar.parser.parse(utils.encodeEntities(processed.code.cleaned) + '\n');

  const data = xyChartDb.getXYChartData();
  const points = (plotData: Point[]): [string, number | string | null][] =>
    plotData.map(([x, y]) => [x, y === undefined ? null : number(y)]);
  return {
    version: 1,
    orientation: xyChartDb.getChartConfig().chartOrientation,
    title: xyChartDb.getDiagramTitle(),
    accTitle: xyChartDb.getAccTitle(),
    accDescr: xyChartDb.getAccDescription(),
    xAxis:
      data.xAxis.type === 'band'
        ? { type: 'band', title: data.xAxis.title, categories: data.xAxis.categories }
        : {
            type: 'linear',
            title: data.xAxis.title,
            min: number(data.xAxis.min),
            max: number(data.xAxis.max),
          },
    yAxis: {
      type: 'linear',
      title: data.yAxis.title,
      min: number(data.yAxis.min),
      max: number(data.yAxis.max),
    },
    plots: data.plots.map((plot) =>
      plot.type === 'line'
        ? {
            type: 'line',
            title: plot.title,
            strokeFill: plot.strokeFill,
            strokeWidth: plot.strokeWidth,
            data: points(plot.data),
            ...('pointLabels' in plot ? { pointLabels: plot.pointLabels } : {}),
          }
        : { type: 'bar', title: plot.title, fill: plot.fill, data: points(plot.data) }
    ),
  };
});
