/**
 * The flowchart DB's canonical model, from the TypeScript reference: the state of `FlowDB` after
 * `flowParser` has parsed a text. The flowchart exporter (`export-flowchart.ts`) and the swimlane
 * exporter (`export-swimlanes.ts`) share it: a swimlane diagram is the flowchart's factory with its
 * own styles, so its parser and its DB are the flowchart's.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the parser; the Jison parser then fills the flowchart DB, and the
 * DB's state is read out as the canonical model. The DB sanitises the node, edge and title texts
 * with DOMPurify, as the reference's `sanitizeText` does, and the port does the same
 * (`ferrigura-syntax`'s `sanitize`), so the reference runs with a window (`provideDomForSanitising`).
 * The two header setters record the value with the whitespace rules the DB applies, and are
 * replaced. See `docs-port/canonical-model.md` and `docs-port/flowchart/PLAN.md`, section 7.
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';

export interface FlowDb {
  clear(): void;
  setAccTitle(text: string): void;
  setAccDescription(text: string): void;
  getDirection(): string | undefined;
  getVertices(): Map<string, Record<string, unknown>>;
  getEdges(): Record<string, unknown>[] & { defaultStyle?: string[]; defaultInterpolate?: string };
  getSubGraphs(): Record<string, unknown>[];
  parser?: unknown;
}

export interface FlowchartModules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  flowParser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  flowDb: { FlowDB: new () => FlowDb };
  common: { default: { sanitizeText: (text: string, config: unknown) => string } };
  utils: { default: { formatUrl: (link: string, config: unknown) => string | undefined } };
}

export async function loadFlowchartModules(reference: Reference): Promise<FlowchartModules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, encode, flowParser, flowDb, common, utils] = await Promise.all([
    reference.load<FlowchartModules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<FlowchartModules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<FlowchartModules['flowParser']>(
      '/packages/mermaid/src/diagrams/flowchart/parser/flowParser.ts'
    ),
    reference.load<FlowchartModules['flowDb']>(
      '/packages/mermaid/src/diagrams/flowchart/flowDb.ts'
    ),
    reference.load<FlowchartModules['common']>('/packages/mermaid/src/diagrams/common/common.ts'),
    reference.load<FlowchartModules['utils']>('/packages/mermaid/src/utils.ts'),
  ]);
  // The header setters are replaced below; the flowchart texts keep the reference's sanitising.
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

/** The canonical model of the flowchart DB after the parse of `text`. Throws when the reference rejects it. */
export function flowchartModel(text: string, modules: FlowchartModules): unknown {
  const { preprocess, encode, flowDb } = modules;
  const flowParser = modules.flowParser.default;
  const { code, title } = preprocess.preprocessDiagram(text);
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
}
