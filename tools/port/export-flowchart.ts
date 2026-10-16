/**
 * Canonical JSON of flowchart diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the parser; the Jison parser then fills the flowchart DB, and the
 * DB's state is read out as the canonical model. The DB sanitises its texts with DOMPurify, which
 * needs a DOM, and the canonical model keeps the source text: for the run the sanitising functions
 * are the identity, and the two header setters record the value with the whitespace rules the DB
 * applies. See `docs-port/canonical-model.md` and `docs-port/flowchart/PLAN.md`, section 7.
 *
 * Usage: `tsx tools/port/export-flowchart.ts inputs.json outputs.json`
 */
import { registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface FlowDb {
  clear(): void;
  setAccTitle(text: string): void;
  setAccDescription(text: string): void;
  getDirection(): string | undefined;
  getVertices(): Map<string, Record<string, unknown>>;
  getEdges(): Record<string, unknown>[] & { defaultStyle?: string[]; defaultInterpolate?: string };
  getSubGraphs(): Record<string, unknown>[];
  parser?: unknown;
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  flowParser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  flowDb: { FlowDB: new () => FlowDb };
  common: { default: { sanitizeText: (text: string, config: unknown) => string } };
  utils: { default: { formatUrl: (link: string, config: unknown) => string | undefined } };
}

async function loadModules(reference: Reference): Promise<Modules> {
  await registerDetectors(reference);
  const [preprocess, encode, flowParser, flowDb, common, utils] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['flowParser']>(
      '/packages/mermaid/src/diagrams/flowchart/parser/flowParser.ts'
    ),
    reference.load<Modules['flowDb']>('/packages/mermaid/src/diagrams/flowchart/flowDb.ts'),
    reference.load<Modules['common']>('/packages/mermaid/src/diagrams/common/common.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
  ]);
  // The DOM-dependent steps are the identity; the canonical model keeps the source text.
  common.default.sanitizeText = (text: string) => text;
  utils.default.formatUrl = (link: string) => link.trim() || undefined;
  return { preprocess, encode, flowParser, flowDb, common, utils };
}

/** The header texts as the grammar gives them, with the DB's whitespace rules applied. */
const headerOf = (db: FlowDb) => {
  const header = { accTitle: '', accDescr: '' };
  db.setAccTitle = (text: string) => {
    header.accTitle = text.replace(/^\s+/g, '');
  };
  db.setAccDescription = (text: string) => {
    header.accDescr = text.replace(/\n\s+/g, '\n');
  };
  return header;
};

await runExporter(loadModules, (input, modules) => {
  const { preprocess, encode, flowDb } = modules;
  const flowParser = modules.flowParser.default;
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const db = new flowDb.FlowDB() as FlowDb & Record<string, unknown>;
  const header = headerOf(db);
  db.clear();
  flowParser.parser.yy = db;
  flowParser.parse(encode.encodeEntities(code.cleaned) + '\n');

  // The DB's private maps, read as they are.
  const classes = db.classes as Map<string, { id: string; styles: string[] }>;
  const tooltips = db.tooltips as Map<string, string>;
  const edges = db.getEdges();

  return {
    version: 1,
    title: title ?? '',
    accTitle: header.accTitle,
    accDescr: header.accDescr,
    direction: db.getDirection() ?? 'TB',
    vertices: [...db.getVertices().values()].map((vertex) => ({
      id: vertex.id,
      text: vertex.text,
      labelType: vertex.labelType,
      shape: vertex.type ?? null,
      styles: vertex.styles,
      classes: vertex.classes,
      link: vertex.link ?? null,
      linkTarget: vertex.linkTarget ?? null,
      props: vertex.props ?? {},
      icon: vertex.icon ?? null,
      form: vertex.form ?? null,
      pos: vertex.pos ?? null,
      img: vertex.img ?? null,
      constraint: vertex.constraint ?? null,
      assetWidth: vertex.assetWidth ?? null,
      assetHeight: vertex.assetHeight ?? null,
    })),
    edges: edges.map((edge) => ({
      id: edge.id,
      isUserDefinedId: edge.isUserDefinedId,
      start: edge.start,
      end: edge.end,
      type: edge.type,
      stroke: edge.stroke,
      length: edge.length ?? null,
      text: edge.text,
      labelType: edge.labelType,
      classes: edge.classes,
      style: edge.style ?? null,
      interpolate: edge.interpolate ?? null,
      animate: edge.animate ?? null,
      animation: edge.animation ?? null,
    })),
    subgraphs: db.getSubGraphs().map((subgraph) => ({
      id: subgraph.id,
      title: subgraph.title,
      labelType: subgraph.labelType,
      nodes: subgraph.nodes,
      dir: subgraph.dir ?? null,
      classes: subgraph.classes,
      metadata: subgraph.metadata ?? {},
    })),
    classes: [...classes.values()].map((cls) => ({ id: cls.id, styles: cls.styles })),
    tooltips: [...tooltips].map(([id, tooltip]) => ({ id, tooltip })),
    defaultStyle: edges.defaultStyle ?? null,
    defaultInterpolate: edges.defaultInterpolate ?? null,
  };
});
