/**
 * Layout data of state diagrams from the TypeScript reference: `StateDB.getData()` after the text is
 * parsed and the draw's second `extract`, with the configuration a render has
 * (`stateRenderer-v3-unified.ts`, `draw`).
 *
 * For each input the configuration is set up as `mermaidAPI` sets it before a render: the
 * configuration is reset, the text goes through `preprocessDiagram`, its diagram type is detected,
 * the diagram scope is set (so `look` resolves to `neo`, `config.ts` `resolveAppearance`) and the
 * directives are added. The parser fills the DB, then `draw` runs `extract(getRootDocV2())` and
 * `getData()` is read out with the names and the shape of the Rust `LayoutData`
 * (`crates/ferrigura-render/src/state/data.rs`).
 *
 * Normalisation, documented here:
 *
 * - an `undefined` field is `null` where the Rust field is an `Option`, and `[]` or `''` where the
 *   layout reads an empty value. These are the readings of the layout, not values of the
 *   reference: a state edge has no `isUserDefinedId`, `type`, `arrowTypeStart` or `cssCompiledStyles`,
 *   and a note edge has no `label`, which the Rust side writes as its empty value;
 * - a node without a `labelType` (a composite's note group) is written as `text`, which is how the
 *   layout reads an undefined label type (`label.rs`: anything but Markdown is plain text), and a
 *   node without a shape (a state first named by a note) is written as the empty shape;
 * - a trailing concurrency region gets `generateId()` (`Math.random`) in the reference, and every
 *   string that holds that id (its DOM id, its children's ids) gets the region's placeholder,
 *   `<generated:k>`, as the parser's model writes it (`state-generated.ts`, `state/db.rs`);
 * - the edge style is a string in the reference, and the Rust side keeps its declarations as a list.
 *
 * Usage: `tsx tools/port/export-state-data.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';
import { generatedPlaceholders, replaceGenerated } from './state-generated.ts';

type Json = Record<string, unknown>;

interface StateDb {
  rootDoc: Json[];
  clear(): void;
  extract(statements: unknown): void;
  getRootDocV2(): unknown;
  getData(): { nodes: Json[]; edges: Json[]; direction: string };
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
  state: {
    diagram: {
      parser: { parser: { yy: unknown }; parse(text: string): unknown };
      db: StateDb;
    };
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, detectType, preprocess, encode, state] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['state']>('/packages/mermaid/src/diagrams/state/stateDiagram-v2.ts'),
  ]);
  return { config, detectType, preprocess, encode, state };
}

/** A string or a number as the reference holds it, or `null` for an undefined field. */
const orNull = (value: unknown): unknown => value ?? null;

/** A list or an object the reference may leave undefined. */
const orEmpty = (value: unknown, empty: unknown): unknown => value ?? empty;

/** A node of `getData()`, with the names of the Rust `Node` and the state fields it adds. */
function nodeModel(node: Json): Json {
  return {
    id: node.id,
    domId: node.domId,
    label: node.label,
    labelType: node.labelType ?? 'text',
    labelStyle: node.labelStyle,
    parentId: orNull(node.parentId),
    padding: node.padding,
    cssStyles: orEmpty(node.cssStyles, []),
    cssCompiledStyles: orEmpty(node.cssCompiledStyles, []),
    cssClasses: node.cssClasses,
    dir: orNull(node.dir),
    look: node.look,
    link: null,
    linkTarget: null,
    tooltip: null,
    icon: null,
    pos: null,
    img: null,
    assetWidth: null,
    assetHeight: null,
    constraint: null,
    isGroup: node.isGroup,
    // A state first named by a note has no shape; the renderer names it `undefined`.
    shape: node.shape ?? '',
    minWidth: orNull(node.minWidth),
    colorIndex: orNull(node.colorIndex),
    metadata: {},
    wrappingWidth: node.wrappingWidth,
    rx: node.rx,
    ry: node.ry,
    type: node.type,
    centerLabel: node.centerLabel,
    position: node.position,
    description: node.description,
  };
}

/** An edge of `getData()`, with the names of the Rust `Edge`; see the normalisation above. */
function edgeModel(edge: Json): Json {
  const style = typeof edge.style === 'string' ? edge.style.split(';').filter(Boolean) : [];
  const labelStyle =
    typeof edge.labelStyle === 'string' && edge.labelStyle ? [edge.labelStyle] : [];
  return {
    id: edge.id,
    isUserDefinedId: edge.isUserDefinedId ?? false,
    start: edge.start,
    end: edge.end,
    type: edge.type ?? '',
    label: edge.label ?? '',
    labelType: edge.labelType,
    labelpos: edge.labelpos,
    thickness: edge.thickness,
    pattern: edge.pattern ?? '',
    minlen: null,
    classes: edge.classes,
    arrowTypeStart: edge.arrowTypeStart ?? '',
    arrowTypeEnd: edge.arrowTypeEnd,
    arrowheadStyle: edge.arrowheadStyle,
    cssCompiledStyles: orEmpty(edge.cssCompiledStyles, []),
    labelStyle,
    style,
    look: edge.look,
    animate: null,
    animation: null,
    curve: null,
  };
}

/**
 * The configuration of a render for the text, then the parse, the draw's `extract` and `getData()`.
 * The scope is left again afterwards, as `mermaidAPI` does at the end of a render.
 */
function layoutOf(text: string, modules: Modules): Json {
  const { config, detectType, preprocess, encode, state } = modules;

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
    const db = state.diagram.db;
    db.clear();
    state.diagram.parser.parser.yy = db;
    state.diagram.parser.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

    // The draw runs `extract` on the root document again, then `getData()`.
    const placeholders = generatedPlaceholders(db.rootDoc);
    db.extract(db.getRootDocV2());
    const { nodes, edges, direction } = db.getData();
    const model: Json = {
      nodes: nodes.map(nodeModel),
      edges: edges.map(edgeModel),
      direction,
      defaultStyle: null,
      defaultInterpolate: null,
    };
    // The placeholder replaces every string that holds a generated id, including the DOM ids.
    return replaceGenerated(model, placeholders);
  } finally {
    config.setDiagramConfigScope(undefined);
  }
}

await runExporter(loadModules, (input, modules) => layoutOf(input.text, modules));
