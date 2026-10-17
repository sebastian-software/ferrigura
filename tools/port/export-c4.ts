/**
 * Canonical JSON of C4 diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram` and `encodeEntities` with a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The DB is cleared and then `init` sets its wrap
 * flag from the effective configuration, as `Diagram.fromText` does, and the parser fills the DB.
 * The model is the DB after the parse: the texts keep the entity placeholders that the DB keeps,
 * a `{ text }` field is its string, and a bare string or an absent field is absent. The common DB's
 * accessibility description is read from its own module, which the C4 `clear()` does not reset, so
 * it is cleared before each input: each diagram is compared as if it were the only one on the page
 * (`docs-port/c4/PLAN.md`, section 5).
 *
 * Usage: `tsx tools/port/export-c4.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type Json = Record<string, unknown>;

/** A text field as the DB holds it: `{ text }`, a bare string, or nothing. */
type TextField = { text: string } | string | null | undefined;

interface Modules {
  config: {
    reset(): void;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: object): void;
    getConfig(): { wrap?: boolean };
  };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; config?: object };
  };
  detectType: { detectType(text: string, config?: unknown): string };
  encode: { encodeEntities(text: string): string };
  /** The Jison parser of `c4Diagram.jison`, whose `yy` is the DB. */
  parser: { yy: unknown; parse(text: string): unknown };
  db: C4Db;
  commonDb: { clear(): void; getAccTitle(): string; getAccDescription(): string };
}

interface C4Db {
  clear(): void;
  setWrap(wrap?: boolean): void;
  getTitle(): string;
  getC4Type(): string;
  getC4ShapeInRow(): number;
  getC4BoundaryInRow(): number;
  getC4ShapeArray(): Json[];
  getBoundaries(): Json[];
  getRels(): Json[];
}

/** The fields of the DB objects that the model names, by object kind. Any other key is extra. */
const SHAPE_KEYS = new Set([
  'alias',
  'label',
  'typeC4Shape',
  'descr',
  'techn',
  'sprite',
  'tags',
  'link',
  'parentBoundary',
  'wrap',
  'bgColor',
  'fontColor',
  'borderColor',
  'shadowing',
  'shape',
  'legendText',
  'legendSprite',
]);

const BOUNDARY_KEYS = new Set([
  'alias',
  'label',
  'type',
  'nodeType',
  'descr',
  'techn',
  'tags',
  'link',
  'sprite',
  'parentBoundary',
  'wrap',
  'bgColor',
  'fontColor',
  'borderColor',
  'shadowing',
  'shape',
  'legendText',
  'legendSprite',
]);

const REL_KEYS = new Set([
  'type',
  'from',
  'to',
  'label',
  'techn',
  'descr',
  'sprite',
  'tags',
  'link',
  'wrap',
  'textColor',
  'lineColor',
  'offsetX',
  'offsetY',
]);

/** The string of a `{ text }` field; a bare string or nothing is absent. */
function textOf(field: TextField): string | undefined {
  return typeof field === 'object' && field !== null ? field.text : undefined;
}

/** An offset as the text of its DB value: `String(value)`, which keeps `NaN`. */
function offsetOf(value: number | string | undefined): string | undefined {
  return value === undefined ? undefined : String(value);
}

/** The DB's extra keys, written as they are: a string, or a `{ text }` object. */
function extras(object: Json, known: Set<string>): Json {
  const out: Json = {};
  for (const [key, value] of Object.entries(object)) {
    if (!known.has(key)) out[key] = value;
  }
  return out;
}

function shapeModel(shape: Json): Json {
  return {
    alias: shape.alias,
    typeC4Shape: textOf(shape.typeC4Shape as TextField),
    label: textOf(shape.label as TextField),
    descr: textOf(shape.descr as TextField),
    techn: textOf(shape.techn as TextField),
    sprite: shape.sprite,
    tags: shape.tags,
    link: shape.link,
    parentBoundary: shape.parentBoundary,
    wrap: shape.wrap === true,
    bgColor: shape.bgColor,
    fontColor: shape.fontColor,
    borderColor: shape.borderColor,
    shadowing: shape.shadowing,
    shape: shape.shape,
    legendText: shape.legendText,
    legendSprite: shape.legendSprite,
    ...extras(shape, SHAPE_KEYS),
  };
}

function boundaryModel(boundary: Json): Json {
  return {
    alias: boundary.alias,
    label: textOf(boundary.label as TextField),
    type: textOf(boundary.type as TextField),
    nodeType: boundary.nodeType,
    descr: textOf(boundary.descr as TextField),
    techn: textOf(boundary.techn as TextField),
    tags: boundary.tags,
    link: boundary.link,
    sprite: boundary.sprite,
    parentBoundary: boundary.parentBoundary,
    wrap: boundary.wrap === true,
    bgColor: boundary.bgColor,
    fontColor: boundary.fontColor,
    borderColor: boundary.borderColor,
    shadowing: boundary.shadowing,
    shape: boundary.shape,
    legendText: boundary.legendText,
    legendSprite: boundary.legendSprite,
    ...extras(boundary, BOUNDARY_KEYS),
  };
}

function relModel(rel: Json): Json {
  return {
    type: rel.type,
    from: rel.from,
    to: rel.to,
    label: textOf(rel.label as TextField),
    techn: textOf(rel.techn as TextField),
    descr: textOf(rel.descr as TextField),
    sprite: rel.sprite,
    tags: rel.tags,
    link: rel.link,
    wrap: rel.wrap === true,
    textColor: rel.textColor,
    lineColor: rel.lineColor,
    offsetX: offsetOf(rel.offsetX as number | string | undefined),
    offsetY: offsetOf(rel.offsetY as number | string | undefined),
    ...extras(rel, REL_KEYS),
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, preprocess, detectType, encode, grammar, db, commonDb] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['parser'] }>(
      '/packages/mermaid/src/diagrams/c4/parser/c4Diagram.jison'
    ),
    reference.load<{ default: C4Db }>('/packages/mermaid/src/diagrams/c4/c4Db.ts'),
    reference.load<Modules['commonDb']>('/packages/mermaid/src/diagrams/common/commonDb.ts'),
  ]);
  return {
    config,
    preprocess,
    detectType,
    encode,
    parser: grammar.parser,
    db: db.default,
    commonDb,
  };
}

await runExporter(loadModules, (input, modules) => {
  const { config, preprocess, detectType, encode, parser, db, commonDb } = modules;

  // The configuration of this input, as `mermaidAPI`'s `processAndSetConfigs` sets it.
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

  // `Diagram.fromText`: the DB is cleared, then `init` sets its wrap flag from the configuration.
  // The C4 DB's `clear` leaves the common accessibility state alone, so it is cleared here.
  commonDb.clear();
  db.clear();
  db.setWrap(config.getConfig().wrap);
  parser.yy = db;
  parser.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

  return {
    version: 1,
    c4Type: db.getC4Type(),
    title: db.getTitle(),
    accTitle: commonDb.getAccTitle(),
    accDescr: commonDb.getAccDescription(),
    c4ShapeInRow: db.getC4ShapeInRow(),
    c4BoundaryInRow: db.getC4BoundaryInRow(),
    shapes: db.getC4ShapeArray().map(shapeModel),
    boundaries: db.getBoundaries().map(boundaryModel),
    rels: db.getRels().map(relModel),
  };
});
