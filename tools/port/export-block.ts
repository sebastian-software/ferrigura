/**
 * Canonical JSON of block diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The parser fills the block DB through its own
 * calls, and the DB's sanitising runs DOMPurify, so the reference runs with a window
 * (`provideDomForSanitising`). The blocks, edges and classes are read from the DB in the order of
 * the renderer (`getBlocks`, `getEdges`, `getClasses`), and the root's columns from `getBlock('root')`.
 *
 * The ids the reference generates at random are replaced by placeholders in the order of the tree,
 * on both sides (`docs-port/block/PLAN.md`, section 7): a generated id is `id-<random>-<n>`, a space
 * cell adds `-<j>`, and a space cell's label is its generated id. `normaliseGenerated` does the
 * same on the Rust model in `crates/ferrigura-syntax/examples/canonical.rs`.
 *
 * Usage: `tsx tools/port/export-block.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** A block as the DB holds it: the fields of the model, and the children of a composite. */
interface BlockObject {
  id?: string;
  type?: string;
  label?: string;
  width?: number;
  widthInColumns?: number;
  columns?: number;
  directions?: string[];
  classes?: string[];
  styles?: string[];
  colorIndex?: number;
  children?: BlockObject[];
}

interface EdgeObject {
  id: string;
  start?: string;
  end: string;
  label: string;
  thickness: string;
  pattern: string;
  directions?: string[];
  arrowTypeEnd: string;
  arrowTypeStart: string;
}

interface ClassObject {
  id: string;
  styles: string[];
  textStyles: string[];
}

interface BlockDb {
  clear(): void;
  getBlocks(): BlockObject[];
  getBlock(id: string): BlockObject | undefined;
  getEdges(): EdgeObject[];
  getClasses(): Map<string, ClassObject>;
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string } };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  /** The Jison parser module of `block.jison`, whose `parser.yy` is the DB. */
  grammar: { parser: { yy: unknown; parse(text: string): unknown } };
  db: { default: BlockDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  // A directive's configuration scope is a diagram type, so the detectors are registered.
  await registerDetectors(reference);
  const [preprocess, utils, grammar, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['grammar']>('/packages/mermaid/src/diagrams/block/parser/block.jison'),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/block/blockDB.ts'),
  ]);
  return { preprocess, utils, grammar, db };
}

/** A block as the model writes it: the DB's fields, with `undefined` left out by `JSON.stringify`. */
function toBlock(block: BlockObject): BlockObject {
  return {
    id: block.id,
    type: block.type,
    label: block.label,
    width: block.width,
    widthInColumns: block.widthInColumns,
    columns: block.columns,
    directions: block.directions,
    classes: block.classes,
    styles: block.styles,
    colorIndex: block.colorIndex,
    children: block.children?.map(toBlock),
  };
}

function toEdge(edge: EdgeObject): EdgeObject {
  return {
    id: edge.id,
    start: edge.start,
    end: edge.end,
    label: edge.label,
    thickness: edge.thickness,
    pattern: edge.pattern,
    directions: edge.directions,
    arrowTypeEnd: edge.arrowTypeEnd,
    arrowTypeStart: edge.arrowTypeStart,
  };
}

/** `id-<random>-<n>`, optionally followed by `-<j>` for a space cell; the base is group one. */
const GENERATED_ID = /^(id-[\da-z]*-\d+)(-\d+)?$/;
/** A generated id on its own, the label of a space cell. */
const GENERATED_BASE = /^id-[\da-z]*-\d+$/;

/** Replaces the generated ids and the labels of space cells by `$genN`, numbered in tree order. */
function normaliseGenerated(blocks: BlockObject[]): void {
  const placeholders = new Map<string, string>();
  const placeholder = (base: string): string => {
    let name = placeholders.get(base);
    if (name === undefined) {
      name = `$gen${placeholders.size + 1}`;
      placeholders.set(base, name);
    }
    return name;
  };
  const visit = (list: BlockObject[]): void => {
    for (const block of list) {
      const match = block.id === undefined ? null : GENERATED_ID.exec(block.id);
      if (match) {
        block.id = placeholder(match[1]!) + (match[2] ?? '');
      }
      if (block.type === 'space' && block.label !== undefined && GENERATED_BASE.test(block.label)) {
        block.label = placeholder(block.label);
      }
      if (block.children) {
        visit(block.children);
      }
    }
  };
  visit(blocks);
}

await runExporter(loadModules, (input, { preprocess, utils, grammar, db }) => {
  const { code } = preprocess.preprocessDiagram(input.text);
  const blockDb = db.default;
  blockDb.clear();
  // `Diagram.fromText` sets the parser's `yy` to the DB, then parses the encoded text and a line break.
  grammar.parser.yy = blockDb;
  grammar.parser.parse(utils.encodeEntities(code.cleaned) + '\n');

  const blocks = blockDb.getBlocks().map(toBlock);
  normaliseGenerated(blocks);
  return {
    version: 1,
    columns: blockDb.getBlock('root')?.columns,
    blocks,
    edges: blockDb.getEdges().map(toEdge),
    classDefs: [...blockDb.getClasses().values()].map((entry) => ({
      id: entry.id,
      styles: entry.styles,
      textStyles: entry.textStyles,
    })),
  };
});
