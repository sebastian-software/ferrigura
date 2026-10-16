/**
 * Layout data of class diagrams from the TypeScript reference: `ClassDB.getData()` after the text
 * is parsed, with the configuration a render has (`classRenderer-v3-unified.ts`, `draw`).
 *
 * For each input the configuration is set up as `mermaidAPI` sets it before a render: the
 * configuration is reset, the text goes through `preprocessDiagram`, its diagram type is detected,
 * the diagram scope is set (so `look` resolves to `neo`) and the directives are added. The class
 * parser then fills the DB, and `getData()` is read out with the names and the shape of the Rust
 * `LayoutData` (`crates/ferrigura-render/src/class/data.rs`):
 *
 * - an `undefined` field is `null`, an undefined list is `[]` and undefined metadata is `{}`; the
 *   fields that a class, an interface or a relation never has are written with the Rust default
 *   (see the header of `class/data.rs`), and the `class` block is written for class nodes only;
 * - a class's `domId` loses the counter that `classDb.ts` appends (`classCounter`, a counter the
 *   reference never resets), so that the run does not depend on the inputs before it;
 * - `startLabelRight` and `endLabelLeft` are written for the relations only, as the reference has
 *   them there;
 * - the fields that only the layout sets (sizes, positions, points) are not written.
 *
 * The header setters of the DB are the real `commonDb` setters (`provideDomForSanitising`), which
 * sanitise with DOMPurify. The texts of the classes and the notes go through the reference's own
 * `sanitizeText` inside the DB.
 *
 * Usage: `tsx tools/port/export-class-data.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type Json = Record<string, unknown>;

interface ClassMemberJs {
  id: string;
  memberType: 'method' | 'attribute';
  visibility: string;
  classifier: string;
  parameters?: string;
  returnType?: string;
  text: string;
}

interface ClassDb {
  clear(): void;
  getDirection(): string;
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
  parser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  classDb: { ClassDB: new () => ClassDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, detectType, preprocess, parser, classDb] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['parser']>(
      '/packages/mermaid/src/diagrams/class/parser/classDiagram.jison'
    ),
    reference.load<Modules['classDb']>('/packages/mermaid/src/diagrams/class/classDb.ts'),
  ]);
  return { config, detectType, preprocess, parser, classDb };
}

/** A string or a number as the reference holds it, or `null` for an undefined field. */
const orNull = (value: unknown): unknown => value ?? null;

/** A list the reference may leave undefined. */
const orEmpty = (value: unknown): unknown => value ?? [];

/** A class member with the names and the nulls of the Rust `ClassMember`. */
const memberOf = (member: ClassMemberJs) => ({
  memberType: member.memberType,
  visibility: member.visibility,
  classifier: member.classifier,
  id: member.id,
  parameters: member.parameters ?? null,
  returnType: member.returnType ?? null,
  text: member.text,
});

/** The DB's class id suffix that `classDb.ts` appends to the DOM id: `classId-<name>-<counter>`. */
const withoutCounter = (domId: string): string => domId.replace(/-\d+$/, '');

/** A node of `getData()` with the names and the nulls of the Rust `Node`. */
function nodeModel(node: Json): Json {
  const isClass = node.shape === 'classBox';
  const domId = isClass ? withoutCounter(node.domId as string) : (node.domId ?? node.id);
  return {
    id: node.id,
    domId,
    label: node.label,
    labelType: node.labelType ?? 'text',
    labelStyle: '',
    parentId: orNull(node.parentId),
    // A class, an interface and a relation have no padding; the Rust field is zero, and left out.
    ...(node.padding !== undefined ? { padding: node.padding } : {}),
    cssStyles: orEmpty(node.cssStyles),
    cssCompiledStyles: [],
    // An undefined `cssClasses` is written as the reference writes it into a class attribute.
    cssClasses: String(node.cssClasses),
    dir: null,
    look: node.look,
    link: orNull(node.link),
    linkTarget: orNull(node.linkTarget),
    tooltip: orNull(node.tooltip),
    icon: null,
    pos: null,
    img: null,
    assetWidth: null,
    assetHeight: null,
    constraint: null,
    isGroup: node.isGroup,
    shape: node.shape,
    minWidth: null,
    colorIndex: orNull(node.colorIndex),
    metadata: {},
    ...(isClass
      ? {
          class: {
            text: node.text,
            annotations: node.annotations,
            members: (node.members as ClassMemberJs[]).map(memberOf),
            methods: (node.methods as ClassMemberJs[]).map(memberOf),
            styles: node.styles,
            parent: orNull(node.parent),
            haveCallback: orNull(node.haveCallback),
          },
        }
      : {}),
  };
}

/** An edge of `getData()` with the names and the nulls of the Rust `Edge`. */
function edgeModel(edge: Json): Json {
  const style = edge.style;
  const isRelation = 'startLabelRight' in edge;
  return {
    id: edge.id,
    isUserDefinedId: false,
    start: edge.start,
    end: edge.end,
    type: edge.type,
    label: edge.label ?? '',
    labelType: edge.labelType ?? 'text',
    labelpos: edge.labelpos ?? '',
    thickness: edge.thickness,
    pattern: edge.pattern,
    minlen: null,
    classes: edge.classes,
    arrowTypeStart: edge.arrowTypeStart,
    arrowTypeEnd: edge.arrowTypeEnd,
    arrowheadStyle: edge.arrowheadStyle,
    cssCompiledStyles: [],
    labelStyle: edge.labelStyle,
    // `classRelation.style || ''` is an empty string when there are no styles.
    style: Array.isArray(style) ? style : style ? [style] : [],
    look: edge.look,
    animate: null,
    animation: null,
    curve: null,
    ...(isRelation
      ? {
          startLabelRight: edge.startLabelRight,
          endLabelLeft: edge.endLabelLeft,
        }
      : {}),
  };
}

/**
 * The configuration of a render for the text, then the parse and `getData()`. The scope is left
 * again afterwards, as `mermaidAPI` does at the end of a render.
 */
function layoutOf(text: string, modules: Modules): Json {
  const { config, detectType, preprocess, parser, classDb } = modules;

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
    const db = new classDb.ClassDB();
    db.clear();
    parser.default.parser.yy = db;
    parser.default.parse(processed.code.cleaned);

    const { nodes, edges } = db.getData();
    return {
      nodes: nodes.map(nodeModel),
      edges: edges.map(edgeModel),
      direction: orNull(db.getDirection()),
      defaultStyle: null,
      defaultInterpolate: null,
    };
  } finally {
    config.setDiagramConfigScope(undefined);
  }
}

await runExporter(loadModules, (input, modules) => layoutOf(input.text, modules));
