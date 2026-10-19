/**
 * Canonical JSON of radar diagrams, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText`: the text is detected, the DB is cleared, the front
 * matter title is set, and the text (with `encodeEntities` and a line break) goes to the parser of
 * `radar/parser.ts`, which fills the radar DB in its own order (the header texts, the axes, the
 * curves, the options). The model is read out of that DB. The DB's header setters sanitise with
 * DOMPurify, which needs a window. A value that is not finite is written as its sentinel string,
 * since `JSON.stringify` would write `null` (`docs-port/canonical-model.md`).
 *
 * Usage: `tsx tools/port/export-radar.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface RadarDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getAxes(): { name: string; label: string }[];
  getCurves(): { name: string; label: string; entries: number[] }[];
  getOptions(): {
    showLegend: boolean;
    ticks: number;
    max: number | null;
    min: number;
    graticule: string;
  };
}

interface Modules {
  detectType: { detectType(text: string): string };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  parser: { parse(text: string): Promise<void> };
  db: RadarDb;
}

interface CanonicalModel {
  version: number;
  title: string;
  accTitle: string;
  accDescr: string;
  axes: { name: string; label: string }[];
  curves: { name: string; label: string; entries: (number | string)[] }[];
  options: {
    showLegend: boolean;
    ticks: number | string;
    max: number | string | null;
    min: number | string;
    graticule: string;
  };
}

/** A number as the canonical model writes it: the number, or the sentinel of a value that is not finite. */
function jsonNumber(value: number): number | string {
  if (Number.isFinite(value)) return value;
  if (Number.isNaN(value)) return 'NaN';
  return value > 0 ? 'Infinity' : '-Infinity';
}

async function loadModules(reference: Reference): Promise<Modules> {
  // DOMPurify takes its window when its module loads, so the window comes first.
  provideDomForSanitising();
  await registerDetectors(reference);
  const [detectType, preprocess, encode, parser, radarDb] = await Promise.all([
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['parser'] }>('/packages/mermaid/src/diagrams/radar/parser.ts'),
    reference.load<{ db: RadarDb }>('/packages/mermaid/src/diagrams/radar/db.ts'),
  ]);
  return {
    detectType,
    preprocess,
    encode,
    parser: parser.parser,
    db: radarDb.db,
  };
}

async function convert(input: { text: string }, modules: Modules): Promise<CanonicalModel> {
  const { detectType, preprocess, encode, parser, db } = modules;

  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned);
  } catch {
    diagramType = undefined;
  }
  if (diagramType !== 'radar') {
    throw new Error(`not a radar diagram (detected: ${diagramType ?? 'none'})`);
  }

  // `Diagram.fromText`: the DB is cleared, and given the front matter title.
  db.clear();
  if (processed.title) {
    db.setDiagramTitle(processed.title);
  }
  const text = encode.encodeEntities(processed.code.cleaned) + '\n';
  await parser.parse(text);

  const options = db.getOptions();
  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    axes: db.getAxes().map((axis) => ({ name: axis.name, label: axis.label })),
    curves: db.getCurves().map((curve) => ({
      name: curve.name,
      label: curve.label,
      entries: curve.entries.map(jsonNumber),
    })),
    options: {
      showLegend: options.showLegend,
      ticks: jsonNumber(options.ticks),
      max: options.max === null ? null : jsonNumber(options.max),
      min: jsonNumber(options.min),
      graticule: options.graticule,
    },
  };
}

await runExporter(loadModules, convert);
