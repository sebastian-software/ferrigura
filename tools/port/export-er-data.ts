/**
 * Layout data of ER diagrams from the TypeScript reference: `ErDB.getData()` after the text is
 * parsed, with the configuration a render has (`erRenderer-unified.ts`, `draw`).
 *
 * For each input the configuration is set up as `mermaidAPI` sets it before a render: the
 * configuration is reset, the text goes through `preprocessDiagram`, its diagram type is detected,
 * the diagram scope is set (so `look` resolves to `neo`, `config.ts` `resolveAppearance`) and the
 * directives are added. The DB then parses the text, and `getData()` is read out with the names and
 * the shape of the Rust `LayoutData` (`crates/ferrigura-render/src/er/data.rs`):
 *
 * - an `undefined` field is `null`, an undefined list is `[]`, undefined metadata is `{}`, and an
 *   undefined flag is `false`. An entity has no `padding` and no `labelStyle`, and an edge has no
 *   `isUserDefinedId`, `labelStyle` or `arrowheadStyle`: the renderer reads none of them for these
 *   nodes and edges as anything but its empty value, so the Rust side holds `0`, `""`, `false` or
 *   `[]` (see the header of `er/data.rs`);
 * - a subgraph node has no `domId`, and the renderer prefixes `node.domId || node.id`
 *   (`rendering-util/render.ts` 86-93), so the id is written, as the Rust field holds it;
 * - an entity carries its `alias` and `attributes` next to the node's fields, as the flattened
 *   `EntityFields` of `crates/ferrigura-render/src/unified/data.rs` has them;
 * - the fields that only the layout sets (sizes, positions, points) are not written.
 *
 * The header texts do not reach `getData`, and their setters sanitise with DOMPurify, which needs a
 * DOM, so the subgraph titles and the entity names are the texts the Rust DB produces with
 * `ferrigura-syntax`'s `sanitize`.
 *
 * Usage: `tsx tools/port/export-er-data.ts inputs.json outputs.json`
 */
import { runExporter } from './exporter.ts';
import {
  provideDomForSanitising,
  registerDetectors,
  registerErDetector,
  type Reference,
} from './reference.ts';

type Json = Record<string, unknown>;

interface ErDb {
  clear(): void;
  getDirection(): string;
  getData(): { nodes: Json[]; edges: Json[]; direction?: string };
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
  erParser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  erDb: { ErDB: new () => ErDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  await registerErDetector(reference);
  const [config, detectType, preprocess, encode, erParser, erDb] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['erParser']>('/packages/mermaid/src/diagrams/er/parser/erDiagram.jison'),
    reference.load<Modules['erDb']>('/packages/mermaid/src/diagrams/er/erDb.ts'),
  ]);
  return { config, detectType, preprocess, encode, erParser, erDb };
}

/** A string or a number as the reference holds it, or `null` for an undefined field. */
const orNull = (value: unknown): unknown => value ?? null;

/** A list or an object the reference may leave undefined. */
const orEmpty = (value: unknown, empty: unknown): unknown => value ?? empty;

/** An attribute of an entity, with the names of the Rust `ErAttribute`. */
function attributeModel(attribute: Json): Json {
  return {
    type: attribute.type,
    name: attribute.name,
    keys: attribute.keys,
    comment: attribute.comment,
  };
}

/** A node of `getData()` with the names and the nulls of the Rust `Node`. */
function nodeModel(node: Json): Json {
  const model: Json = {
    id: node.id,
    domId: node.domId ?? node.id,
    label: node.label,
    labelType: node.labelType,
    labelStyle: node.labelStyle ?? '',
    parentId: orNull(node.parentId),
    padding: node.padding ?? 0,
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
  // An entity keeps its alias and attributes, which only the entity nodes have.
  if (node.attributes !== undefined) {
    model.alias = node.alias;
    model.attributes = (node.attributes as Json[]).map(attributeModel);
  }
  return model;
}

/** An edge of `getData()` with the names and the nulls of the Rust `Edge`. */
function edgeModel(edge: Json): Json {
  return {
    id: edge.id,
    isUserDefinedId: edge.isUserDefinedId ?? false,
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
    arrowheadStyle: edge.arrowheadStyle ?? '',
    cssCompiledStyles: orEmpty(edge.cssCompiledStyles, []),
    labelStyle: orEmpty(edge.labelStyle, []),
    style: orEmpty(edge.style, []),
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
  const { config, detectType, preprocess, encode, erParser, erDb } = modules;

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
    const db = new erDb.ErDB();
    db.clear();
    erParser.default.parser.yy = db;
    erParser.default.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

    const { nodes, edges, direction } = db.getData();
    return {
      nodes: nodes.map(nodeModel),
      edges: edges.map(edgeModel),
      direction: orNull(direction ?? db.getDirection()),
      defaultStyle: null,
      defaultInterpolate: null,
    };
  } finally {
    config.setDiagramConfigScope(undefined);
  }
}

await runExporter(loadModules, (input, modules) => layoutOf(input.text, modules));
