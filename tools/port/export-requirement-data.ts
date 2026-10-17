/**
 * Layout data of requirement diagrams from the TypeScript reference: `RequirementDB.getData()`
 * after the text is parsed, with the configuration a render has (`requirementRenderer.ts`, `draw`).
 *
 * For each input the configuration is set up as `mermaidAPI` sets it before a render: the
 * configuration is reset, the text goes through `preprocessDiagram`, its diagram type is detected,
 * the diagram scope is set (so `look` resolves to `neo`) and the directives are added. The Jison
 * parser fills the DB as `Diagram.fromText` gives it the text, and `getData()` is read out with the
 * names and the shape of the Rust `LayoutData` (`crates/ferrigura-render/src/requirement/data.rs`):
 *
 * - an `undefined` field is `null`, an undefined list is `[]`, undefined metadata is `{}`, and an
 *   undefined label is `''` and an undefined label type is `'text'` (a requirement has neither);
 * - a requirement or an element carries its own fields in a `requirement` object, as the Rust
 *   `RequirementFields` has them; an element has no `verifyMethod` and a requirement no `docRef`;
 * - an endpoint that names no requirement and no element is `undefined` in the reference; the Rust
 *   side keeps the empty name, so the undefined endpoint is written as `''`;
 * - the fields that only the layout sets (sizes, positions, points) are not written.
 *
 * Usage: `tsx tools/port/export-requirement-data.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type Json = Record<string, unknown>;

interface RequirementDb {
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
  grammar: { yy: unknown; parse(text: string): unknown };
  db: { RequirementDB: new () => RequirementDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, detectType, preprocess, encode, grammar, db] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/requirement/parser/requirementDiagram.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/requirement/requirementDb.ts'),
  ]);
  return { config, detectType, preprocess, encode, grammar: grammar.parser, db };
}

/** A string or a number as the reference holds it, or `null` for an undefined field. */
const orNull = (value: unknown): unknown => value ?? null;

/** The requirement's or the element's own fields, with the names of the Rust `RequirementFields`. */
function requirementFieldsOf(node: Json): Json {
  if ('verifyMethod' in node) {
    return {
      name: node.name,
      type: node.type,
      requirementId: node.requirementId,
      text: node.text,
      risk: node.risk,
      verifyMethod: node.verifyMethod,
    };
  }
  return { name: node.name, type: node.type, docRef: node.docRef };
}

/** A node of `getData()` with the names and the nulls of the Rust `Node`. */
function nodeModel(node: Json): Json {
  return {
    id: node.id,
    domId: node.domId ?? node.id,
    label: node.label ?? '',
    labelType: node.labelType ?? 'text',
    labelStyle: node.labelStyle ?? '',
    parentId: null,
    padding: node.padding ?? 0,
    cssStyles: node.cssStyles ?? [],
    cssCompiledStyles: [],
    cssClasses: String(node.cssClasses),
    dir: null,
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
    isGroup: false,
    shape: node.shape,
    minWidth: null,
    colorIndex: orNull(node.colorIndex),
    metadata: {},
    requirement: requirementFieldsOf(node),
  };
}

/** An edge of `getData()` with the names and the nulls of the Rust `Edge`. */
function edgeModel(edge: Json): Json {
  return {
    id: edge.id,
    isUserDefinedId: false,
    start: edge.start ?? '',
    end: edge.end ?? '',
    type: edge.type,
    label: edge.label,
    labelType: edge.labelType,
    labelpos: edge.labelpos,
    thickness: edge.thickness,
    pattern: edge.pattern,
    minlen: null,
    classes: edge.classes,
    arrowTypeStart: edge.arrowTypeStart,
    arrowTypeEnd: edge.arrowTypeEnd,
    arrowheadStyle: '',
    cssCompiledStyles: [],
    labelStyle: [],
    style: edge.style,
    look: edge.look,
    animate: null,
    animation: null,
    curve: null,
  };
}

/**
 * The configuration of a render for the text, then the parse and `getData()`. The scope is left
 * again afterwards, as `mermaidAPI` does at the end of a render.
 */
function layoutOf(text: string, modules: Modules): Json {
  const { config, detectType, preprocess, encode, grammar, db } = modules;

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
    const requirementDb = new db.RequirementDB();
    requirementDb.clear();
    grammar.yy = requirementDb;
    grammar.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

    const { nodes, edges, direction } = requirementDb.getData();
    return {
      nodes: nodes.map(nodeModel),
      edges: edges.map(edgeModel),
      direction: orNull(direction ?? requirementDb.getDirection()),
      defaultStyle: null,
      defaultInterpolate: null,
    };
  } finally {
    config.setDiagramConfigScope(undefined);
  }
}

await runExporter(loadModules, (input, modules) => layoutOf(input.text, modules));
