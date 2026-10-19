/**
 * Canonical JSON of wardley maps, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText`: the text is detected, the DB is cleared, the front
 * matter title is set, and the text (with `encodeEntities` and a line break) goes to the Langium
 * parser of `wardleyParser.ts`, which fills the wardley DB in its own order. The model is the DB
 * state after the parse, as `getWardleyData` gives it (there is no layout layer), with the header
 * texts of the common DB. A non-finite number is its sentinel string, as `wardley.rs` writes it.
 *
 * Usage: `tsx tools/port/export-wardley.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** A node of the DB's build result, with the fields the populate calls set. */
interface WardleyNodeData {
  id: string;
  label: string;
  x: number;
  y: number;
  className?: string;
  labelOffsetX?: number;
  labelOffsetY?: number;
  inertia?: boolean;
  sourceStrategy?: string;
  inPipeline?: boolean;
  isPipelineParent?: boolean;
}

interface WardleyData {
  nodes: WardleyNodeData[];
  links: { source: string; target: string; dashed?: boolean; label?: string; flow?: string }[];
  trends: { nodeId: string; targetX: number; targetY: number }[];
  pipelines: { nodeId: string; componentIds: string[] }[];
  annotations: { number: number; coordinates: { x: number; y: number }[]; text?: string }[];
  notes: { text: string; x: number; y: number }[];
  accelerators: { name: string; x: number; y: number }[];
  deaccelerators: { name: string; x: number; y: number }[];
  annotationsBox?: { x: number; y: number };
  axes: { stages?: string[]; stageBoundaries?: number[] };
  size?: { width: number; height: number };
}

interface WardleyDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getWardleyData(): WardleyData;
}

interface WardleyParser {
  parser: { yy?: unknown };
  parse(text: string): Promise<void>;
}

interface Modules {
  detectType: { detectType(text: string): string };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  parser: WardleyParser;
  db: WardleyDb;
}

/** A number as the model writes it: the JSON number, or the sentinel string when it is not finite. */
function num(value: number): number | string {
  if (Number.isFinite(value)) return value;
  if (Number.isNaN(value)) return 'NaN';
  return value > 0 ? 'Infinity' : '-Infinity';
}

/** A value that the DB leaves `undefined` is `null` in the model. */
function orNull<T, U>(value: T | undefined, map: (item: T) => U): U | null {
  return value === undefined ? null : map(value);
}

async function loadModules(reference: Reference): Promise<Modules> {
  // DOMPurify takes its window when its module loads, so the window comes first.
  provideDomForSanitising();
  await registerDetectors(reference);
  const [detectType, preprocess, encode, parser, db] = await Promise.all([
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: WardleyParser }>(
      '/packages/mermaid/src/diagrams/wardley/wardleyParser.ts'
    ),
    reference.load<{ default: WardleyDb }>('/packages/mermaid/src/diagrams/wardley/wardleyDb.ts'),
  ]);
  return { detectType, preprocess, encode, parser: parser.parser, db: db.default };
}

async function convert(input: { text: string }, modules: Modules) {
  const { detectType, preprocess, encode, parser, db } = modules;

  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned);
  } catch {
    diagramType = undefined;
  }
  if (diagramType !== 'wardley') {
    throw new Error(`not a wardley map (detected: ${diagramType ?? 'none'})`);
  }

  // `Diagram.fromText`: the DB is cleared, then given the front matter title.
  db.clear();
  if (processed.title) {
    db.setDiagramTitle(processed.title);
  }
  parser.parser.yy = db;
  const text = encode.encodeEntities(processed.code.cleaned) + '\n';
  await parser.parse(text);

  const data = db.getWardleyData();
  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    size: orNull(data.size, (size) => ({ width: num(size.width), height: num(size.height) })),
    axes: {
      stages: data.axes.stages ?? null,
      stageBoundaries: orNull(data.axes.stageBoundaries, (boundaries) => boundaries.map(num)),
    },
    nodes: data.nodes.map((node) => ({
      id: node.id,
      label: node.label,
      x: node.x,
      y: node.y,
      className: node.className ?? null,
      labelOffsetX: orNull(node.labelOffsetX, num),
      labelOffsetY: orNull(node.labelOffsetY, num),
      inertia: node.inertia ?? null,
      sourceStrategy: node.sourceStrategy ?? null,
      inPipeline: node.inPipeline ?? null,
      isPipelineParent: node.isPipelineParent ?? null,
    })),
    links: data.links.map((link) => ({
      source: link.source,
      target: link.target,
      dashed: link.dashed === true,
      label: link.label ?? null,
      flow: link.flow ?? null,
    })),
    trends: data.trends.map((trend) => ({
      nodeId: trend.nodeId,
      targetX: trend.targetX,
      targetY: trend.targetY,
    })),
    pipelines: data.pipelines.map((pipeline) => ({
      nodeId: pipeline.nodeId,
      componentIds: [...pipeline.componentIds],
    })),
    annotations: data.annotations.map((annotation) => ({
      number: num(annotation.number),
      coordinates: annotation.coordinates.map((point) => ({ x: point.x, y: point.y })),
      text: annotation.text ?? '',
    })),
    notes: data.notes.map((note) => ({ text: note.text, x: note.x, y: note.y })),
    accelerators: data.accelerators.map((mark) => ({ name: mark.name, x: mark.x, y: mark.y })),
    deaccelerators: data.deaccelerators.map((mark) => ({
      name: mark.name,
      x: mark.x,
      y: mark.y,
    })),
    annotationsBox: orNull(data.annotationsBox, (point) => ({ x: point.x, y: point.y })),
  };
}

await runExporter(loadModules, convert);
