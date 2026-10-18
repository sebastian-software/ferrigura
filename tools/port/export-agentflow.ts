/**
 * Canonical JSON of agentflow diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram` as `Diagram.fromText` gives it to the parser: the
 * text with its comments (`withComments`, which the agentflow DB asks for), unless it is longer
 * than `maxTextSize`, then `encodeEntities` and a line break. The parser is the wrapper of
 * `agentflowParser.ts`, whose `yy` is a new DB for each input. The DB's sanitising runs DOMPurify,
 * so the reference runs with a window (`provideDomForSanitising`).
 *
 * The model is the DB state after the parse (`docs-port/agentflow/NOTES.md`, section 4.1). Each
 * record is written with the fields of the model only, so `JSON.stringify` leaves out the keys the
 * reference leaves `undefined`, as the model does. A number that is not finite is written as its
 * sentinel string, as `value.rs` writes it, since `JSON.stringify` would write `null`.
 *
 * Usage: `tsx tools/port/export-agentflow.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** The reference's `maxTextSize` default (`Diagram.ts`), in UTF-16 units. */
const MAX_TEXT_SIZE = 50000;

type Json = unknown;

interface AgentflowDb {
  clear(): void;
  getVertices(): Map<string, Record<string, unknown>>;
  getEdges(): Record<string, unknown>[];
  getSubGraphs(): Record<string, unknown>[];
  getConnectors(): Record<string, unknown>[];
  getClasses(): Map<string, Record<string, unknown>>;
  getDirection(): string | undefined;
  tooltips: Map<string, string>;
}

type AgentflowDbClass = new () => AgentflowDb;

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string; withComments: string };
      title?: string;
    };
  };
  utils: { encodeEntities(text: string): string };
  parser: { parser: { yy: unknown }; parse(text: string): unknown };
  db: { AgentFlowDB: AgentflowDbClass };
  commonDb: {
    setDiagramTitle(text: string): void;
    getAccTitle(): string;
    getAccDescription(): string;
    getDiagramTitle(): string;
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, parser, db, commonDb] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ default: Modules['parser'] }>(
      '/packages/mermaid/src/diagrams/agentflow/parser/agentflowParser.ts'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/agentflow/agentflowDb.ts'),
    reference.load<Modules['commonDb']>('/packages/mermaid/src/diagrams/common/commonDb.ts'),
  ]);
  return { preprocess, utils, parser: parser.default, db, commonDb };
}

/** A number that is not finite is its sentinel string; other values pass as they are. */
function sentinels(value: Json): Json {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : String(value);
  }
  if (Array.isArray(value)) {
    return value.map(sentinels);
  }
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, sentinels(entry)]));
  }
  return value;
}

/** The vertex fields of the model, in the reference's own names. */
function vertexJson(vertex: Record<string, unknown>): Json {
  return sentinels({
    id: vertex.id,
    text: vertex.text,
    labelType: vertex.labelType,
    type: vertex.type,
    styles: vertex.styles,
    classes: vertex.classes,
    props: vertex.props,
    metadata: vertex.metadata,
    link: vertex.link,
    linkTarget: vertex.linkTarget,
    isConnector: vertex.isConnector,
  });
}

/** The edge fields of the model. */
function edgeJson(edge: Record<string, unknown>): Json {
  return sentinels({
    id: edge.id,
    isUserDefinedId: edge.isUserDefinedId,
    start: edge.start,
    end: edge.end,
    type: edge.type,
    stroke: edge.stroke,
    length: edge.length,
    text: edge.text,
    labelType: edge.labelType,
    classes: edge.classes,
    style: edge.style,
    interpolate: edge.interpolate,
    animate: edge.animate,
    animation: edge.animation,
    edgeSemantic: edge.edgeSemantic,
    metadata: edge.metadata,
  });
}

/** The container fields of the model. */
function subGraphJson(subGraph: Record<string, unknown>): Json {
  return sentinels({
    id: subGraph.id,
    nodes: subGraph.nodes,
    title: subGraph.title,
    labelType: subGraph.labelType,
    dir: subGraph.dir,
    type: subGraph.type,
    classes: subGraph.classes,
    metadata: subGraph.metadata,
  });
}

await runExporter(loadModules, (input, { preprocess, utils, parser, db, commonDb }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const parsed = new db.AgentFlowDB();
  parsed.clear();
  // `Diagram.fromText` sets the front matter title after the clear, before the parse.
  if (title) {
    commonDb.setDiagramTitle(title);
  }
  const source = code.withComments.length > MAX_TEXT_SIZE ? code.cleaned : code.withComments;
  // The wrapper of `agentflowParser.ts` is the parser `Diagram.fromText` calls: it strips the blanks
  // between a closing brace and a line break before the Jison parser reads the text.
  parser.parser.yy = parsed;
  parser.parse(utils.encodeEntities(source) + '\n');

  const vertices = [...parsed.getVertices().values()];
  const connectors = parsed.getConnectors();
  return {
    version: 1,
    title: commonDb.getDiagramTitle(),
    accTitle: commonDb.getAccTitle(),
    accDescr: commonDb.getAccDescription(),
    direction: parsed.getDirection(),
    vertices: vertices.map(vertexJson),
    edges: parsed.getEdges().map(edgeJson),
    subGraphs: parsed.getSubGraphs().map(subGraphJson),
    connectors: connectors.map(vertexJson),
    classes: [...parsed.getClasses().values()].map((entry) =>
      sentinels({ id: entry.id, styles: entry.styles, textStyles: entry.textStyles })
    ),
    tooltips: Object.fromEntries(parsed.tooltips),
  };
});
