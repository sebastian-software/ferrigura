/**
 * Layout data of flowchart diagrams from the TypeScript reference: `FlowDB.getData()` after the
 * text is parsed, with the configuration a render has (`flowRenderer-v3-unified.ts`, `draw`).
 *
 * For each input the configuration is set up as `mermaidAPI` sets it before a render: the
 * configuration is reset, the text goes through `preprocessDiagram`, its diagram type is detected,
 * the diagram scope is set (so `look` resolves to `neo`, `config.ts` `resolveAppearance`) and the
 * directives are added. The DB then parses the text, and `getData()` is read out with the names and
 * the shape of the Rust `LayoutData` (`crates/ferrigura-render/src/flowchart/data.rs`):
 *
 * - an `undefined` field is `null`; an undefined list is `[]` and undefined metadata is `{}`;
 * - a subgraph node has no `domId`, and the renderer prefixes `node.domId || node.id`
 *   (`rendering-util/render.ts` 86-93), so the id is written, as the Rust field holds it;
 * - the fields that only the layout sets (sizes, positions, points, the arrowhead) are not written,
 *   as they are not serialised on the Rust side.
 *
 * The reference's `sanitizeText` runs as it does (DOMPurify, with a window from
 * `provideDomForSanitising`), so the labels are the sanitised texts that the Rust DB produces with
 * `ferrigura-syntax`'s `sanitize`. A link is only trimmed; `formatUrl` needs no DOM for these inputs.
 *
 * Usage: `tsx tools/port/export-flowchart-data.ts inputs.json outputs.json`
 */
import { runExporter } from './exporter.ts';
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';

type Json = Record<string, unknown>;

interface FlowDb {
  clear(): void;
  setAccTitle(text: string): void;
  setAccDescription(text: string): void;
  getDirection(): string | undefined;
  getEdges(): Json[] & { defaultStyle?: string[]; defaultInterpolate?: string };
  getData(): { nodes: Json[]; edges: Json[] };
}

interface Modules {
  config: {
    setSiteConfig(conf: Json): unknown;
    reset(): void;
    getConfig(): Json;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Json): void;
  };
  detectType: {
    detectType(text: string, config?: Json): string;
  };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; config?: Json };
  };
  encode: { encodeEntities(text: string): string };
  flowParser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  flowDb: { FlowDB: new () => FlowDb };
  common: { default: { sanitizeText: (text: string, config: unknown) => string } };
  utils: { default: { formatUrl: (link: string, config: unknown) => string | undefined } };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, detectType, preprocess, encode, flowParser, flowDb, common, utils] =
    await Promise.all([
      reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
      reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
      reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
      reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
      reference.load<Modules['flowParser']>(
        '/packages/mermaid/src/diagrams/flowchart/parser/flowParser.ts'
      ),
      reference.load<Modules['flowDb']>('/packages/mermaid/src/diagrams/flowchart/flowDb.ts'),
      reference.load<Modules['common']>('/packages/mermaid/src/diagrams/common/common.ts'),
      reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    ]);
  // A link is only trimmed; see the header.
  utils.default.formatUrl = (link: string) => link.trim() || undefined;
  return { config, detectType, preprocess, encode, flowParser, flowDb, common, utils };
}

/** A string or a number as the reference holds it, or `null` for an undefined field. */
const orNull = (value: unknown): unknown => value ?? null;

/** A list or an object the reference may leave undefined. */
const orEmpty = (value: unknown, empty: unknown): unknown => value ?? empty;

/** A node of `getData()` with the names and the nulls of the Rust `Node`. */
function nodeModel(node: Json): Json {
  return {
    id: node.id,
    domId: node.domId ?? node.id,
    label: node.label,
    labelType: node.labelType,
    labelStyle: node.labelStyle,
    parentId: orNull(node.parentId),
    padding: node.padding,
    cssStyles: orEmpty(node.cssStyles, []),
    cssCompiledStyles: orEmpty(node.cssCompiledStyles, []),
    cssClasses: node.cssClasses,
    dir: orNull(node.dir),
    look: node.look,
    link: orNull(node.link),
    linkTarget: orNull(node.linkTarget),
    tooltip: orNull(node.tooltip),
    icon: orNull(node.icon),
    pos: orNull(node.pos),
    img: orNull(node.img),
    assetWidth: orNull(node.assetWidth),
    assetHeight: orNull(node.assetHeight),
    constraint: orNull(node.constraint),
    isGroup: node.isGroup,
    shape: node.shape,
    minWidth: orNull(node.minWidth),
    colorIndex: orNull(node.colorIndex),
    metadata: orEmpty(node.metadata, {}),
  };
}

/** An edge of `getData()` with the names and the nulls of the Rust `Edge`. */
function edgeModel(edge: Json): Json {
  return {
    id: edge.id,
    isUserDefinedId: edge.isUserDefinedId,
    start: edge.start,
    end: edge.end,
    type: edge.type,
    label: edge.label,
    labelType: edge.labelType,
    labelpos: edge.labelpos,
    thickness: edge.thickness,
    pattern: edge.pattern,
    minlen: orNull(edge.minlen),
    classes: edge.classes,
    arrowTypeStart: edge.arrowTypeStart,
    arrowTypeEnd: edge.arrowTypeEnd,
    arrowheadStyle: edge.arrowheadStyle,
    cssCompiledStyles: orEmpty(edge.cssCompiledStyles, []),
    labelStyle: edge.labelStyle,
    style: edge.style,
    look: edge.look,
    animate: orNull(edge.animate),
    animation: orNull(edge.animation),
    curve: orNull(edge.curve),
  };
}

/**
 * The configuration of a render for the text, then the parse and `getData()`. The scope is left
 * again afterwards, as `mermaidAPI` does at the end of a render.
 */
function layoutOf(text: string, modules: Modules): Json {
  const { config, detectType, preprocess, encode, flowParser, flowDb } = modules;

  config.setSiteConfig({});
  config.reset();
  const processed = preprocess.preprocessDiagram(text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
  } catch {
    diagramType = undefined;
  }
  config.setDiagramConfigScope(diagramType);
  config.addDirective(processed.config ?? {});
  try {
    const db = new flowDb.FlowDB();
    db.clear();
    // The header texts do not reach getData, and their setters sanitise with DOMPurify, which needs
    // a DOM. They are replaced, as export-flowchart.ts does.
    db.setAccTitle = () => undefined;
    db.setAccDescription = () => undefined;
    const parser = flowParser.default;
    parser.parser.yy = db;
    parser.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

    const { nodes, edges } = db.getData();
    const edgeList = db.getEdges();
    return {
      nodes: nodes.map(nodeModel),
      edges: edges.map(edgeModel),
      direction: orNull(db.getDirection()),
      defaultStyle: orNull(edgeList.defaultStyle),
      defaultInterpolate: orNull(edgeList.defaultInterpolate),
    };
  } finally {
    config.setDiagramConfigScope(undefined);
  }
}

await runExporter(loadModules, (input, modules) => layoutOf(input.text, modules));
