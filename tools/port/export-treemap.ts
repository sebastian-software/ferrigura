/**
 * Canonical JSON of treemap diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Langium parser. The reference makes a new DB for each diagram
 * (`get db()` in `diagram.ts`), clears it, sets the front matter title, and runs the parser, which
 * fills the DB through `populate`. The header texts go through DOMPurify, so the reference runs
 * with a window (`provideDomForSanitising`). The model is the DB after the parse: the tree of
 * `buildHierarchy` under the root, the classes, and the header texts. A non-finite value is written
 * as its sentinel string, as `value.rs` writes it.
 *
 * Usage: `tsx tools/port/export-treemap.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** A node of the hierarchy: a section has `children` (possibly empty), a leaf has `value`. */
interface ReferenceNode {
  name: string;
  children?: ReferenceNode[];
  value?: number;
  classSelector?: string;
  cssCompiledStyles?: string[];
}

interface ReferenceClass {
  id: string;
  styles: string[];
  textStyles: string[];
}

interface TreeMapDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getClasses(): Map<string, ReferenceClass>;
  getRoot(): { name: string; children: ReferenceNode[] };
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: { encodeEntities(text: string): string };
  /** The Langium parser of `parser.ts`, whose `parser.yy` is the DB. */
  parser: { parser: { yy: unknown }; parse(text: string): Promise<void> };
  db: { TreeMapDB: new () => TreeMapDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, parser, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['parser'] }>(
      '/packages/mermaid/src/diagrams/treemap/parser.ts'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/treemap/db.ts'),
  ]);
  return { preprocess, utils, parser: parser.parser, db };
}

/** A value as JSON: the number when it is finite, else `"NaN"`, `"Infinity"` or `"-Infinity"`. */
const number = (value: number): number | string => (Number.isFinite(value) ? value : String(value));

/** A node as JSON. An absent `children`, `classSelector` or style list is left out. */
function node(item: ReferenceNode): Record<string, unknown> {
  const out: Record<string, unknown> = { name: item.name };
  if (item.children !== undefined) out.children = item.children.map(node);
  if (item.value !== undefined) out.value = number(item.value);
  if (item.classSelector !== undefined) out.classSelector = item.classSelector;
  if (item.cssCompiledStyles !== undefined) out.cssCompiledStyles = item.cssCompiledStyles;
  return out;
}

await runExporter(loadModules, async (input, { preprocess, utils, parser, db }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const treemapDb = new db.TreeMapDB();
  parser.parser.yy = treemapDb;
  treemapDb.clear();
  // `Diagram.fromText` sets the front matter title before the parse.
  if (title) {
    treemapDb.setDiagramTitle(title);
  }
  await parser.parse(utils.encodeEntities(code.cleaned) + '\n');

  return {
    version: 1,
    title: treemapDb.getDiagramTitle(),
    accTitle: treemapDb.getAccTitle(),
    accDescr: treemapDb.getAccDescription(),
    classes: [...treemapDb.getClasses().values()].map((styleClass) => ({
      id: styleClass.id,
      styles: styleClass.styles,
      textStyles: styleClass.textStyles,
    })),
    tree: { name: '', children: treemapDb.getRoot().children.map(node) },
  };
});
