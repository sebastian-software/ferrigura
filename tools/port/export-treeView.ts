/**
 * Canonical JSON of tree view diagrams, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText`: the text is preprocessed and detected, the DB is
 * cleared, the front matter title is set, and the text (with `encodeEntities` and a line break) goes
 * to the treeView parser, which runs the box-drawing preprocessor and then the Langium grammar and
 * fills the DB in the order of `parser.ts`. The model is the DB after that parse: the texts, the
 * count and the root with its tree. The DB's setters sanitise with DOMPurify, which needs a window.
 *
 * Usage: `tsx tools/port/export-treeView.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface TreeNodeDb {
  id: number;
  level: number;
  name: string;
  nodeType: 'file' | 'directory';
  icon?: string;
  cssClass?: string;
  description?: string;
  children: TreeNodeDb[];
}

interface TreeViewDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getRoot(): TreeNodeDb;
  getCount(): number;
}

interface TreeViewParser {
  parse(text: string): Promise<void>;
}

interface Modules {
  detectType: { detectType(text: string): string };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  parser: TreeViewParser;
  db: TreeViewDb;
}

interface CanonicalNode {
  id: number;
  level: number;
  name: string;
  nodeType: 'file' | 'directory';
  icon: string;
  cssClass: string;
  description: string;
  children: CanonicalNode[];
}

interface CanonicalModel {
  version: number;
  title: string;
  accTitle: string;
  accDescr: string;
  count: number;
  root: CanonicalNode;
}

async function loadModules(reference: Reference): Promise<Modules> {
  // DOMPurify takes its window when its module loads, so the window comes first.
  provideDomForSanitising();
  await registerDetectors(reference);
  const [detectType, preprocess, encode, parser, treeView] = await Promise.all([
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: TreeViewParser }>('/packages/mermaid/src/diagrams/treeView/parser.ts'),
    reference.load<{ default: TreeViewDb }>('/packages/mermaid/src/diagrams/treeView/db.ts'),
  ]);
  return {
    detectType,
    preprocess,
    encode,
    parser: parser.parser,
    db: treeView.default,
  };
}

function canonicalNode(node: TreeNodeDb): CanonicalNode {
  return {
    id: node.id,
    level: node.level,
    name: node.name,
    nodeType: node.nodeType,
    icon: node.icon ?? '',
    cssClass: node.cssClass ?? '',
    description: node.description ?? '',
    children: node.children.map(canonicalNode),
  };
}

async function convert(input: { text: string }, modules: Modules): Promise<CanonicalModel> {
  const { detectType, preprocess, encode, parser, db } = modules;

  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned);
  } catch {
    diagramType = undefined;
  }
  if (diagramType !== 'treeView') {
    throw new Error(`not a treeView diagram (detected: ${diagramType ?? 'none'})`);
  }

  // `Diagram.fromText`: the DB is cleared, and given the front matter title.
  db.clear();
  if (processed.title) {
    db.setDiagramTitle(processed.title);
  }
  const text = encode.encodeEntities(processed.code.cleaned) + '\n';
  await parser.parse(text);

  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    count: db.getCount(),
    root: canonicalNode(db.getRoot()),
  };
}

await runExporter(loadModules, convert);
