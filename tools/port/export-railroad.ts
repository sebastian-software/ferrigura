/**
 * Canonical JSON of railroad diagrams, from the TypeScript reference.
 *
 * Each input goes through `preprocessDiagram` and `Diagram.fromText`, as `mermaidAPI.parse` gives it:
 * the reference's own loader makes the DB of the detected syntax, clears it, sets the front matter
 * title (which the parser's own clear then drops), and runs the Langium parser of that syntax on the
 * cleaned text with its line break. The four syntaxes share one DB and one model, so the model is the
 * DB state after that parse, read out with the rule expressions as the canonical JSON writes them.
 * The texts keep the entity placeholders the DB keeps. A repetition's bounds are numbers, with
 * `Infinity` written as its sentinel string, as the other models write non-finite numbers.
 *
 * Usage: `tsx tools/port/export-railroad.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** The four detector ids of the railroad syntaxes (`railroadDiagram.ts` and the other three). */
const railroadTypes = ['railroad', 'railroadEbnf', 'railroadAbnf', 'railroadPeg'];

/** The AST node of `railroadTypes.ts`, as the DB holds it after `sanitizeAstNode`. */
interface AstNode {
  type: string;
  value?: string;
  name?: string;
  text?: string;
  elements?: AstNode[];
  alternatives?: AstNode[];
  element?: AstNode;
  min?: number;
  max?: number;
}

interface RailroadDb {
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getRules(): { name: string; definition: AstNode }[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: unknown; title?: string };
  };
  diagram: {
    Diagram: {
      fromText(
        code: unknown,
        metadata: { title?: string }
      ): Promise<{ type: string; db: RailroadDb }>;
    };
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, diagram] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['diagram']>('/packages/mermaid/src/Diagram.ts'),
  ]);
  return { preprocess, diagram };
}

/** A count as JSON: the number when it is finite, else its sentinel string. */
const count = (value: number | undefined): number | string => {
  if (value === undefined) {
    throw new Error('a repetition has no bound');
  }
  return Number.isFinite(value) ? value : String(value);
};

/** An expression of the canonical model, from an AST node. */
function expression(node: AstNode): unknown {
  switch (node.type) {
    case 'terminal':
      return { type: 'terminal', value: node.value };
    case 'nonterminal':
      return { type: 'nonterminal', name: node.name };
    case 'special':
      return { type: 'special', text: node.text };
    case 'sequence':
      return { type: 'sequence', elements: (node.elements ?? []).map(expression) };
    case 'choice':
      return { type: 'choice', alternatives: (node.alternatives ?? []).map(expression) };
    case 'optional':
      return { type: 'optional', element: expression(requireChild(node.element)) };
    case 'repetition':
      return {
        type: 'repetition',
        element: expression(requireChild(node.element)),
        min: count(node.min),
        max: count(node.max),
      };
    default:
      throw new Error(`unknown node ${node.type}`);
  }
}

function requireChild(node: AstNode | undefined): AstNode {
  if (!node) {
    throw new Error('a node has no element');
  }
  return node;
}

await runExporter(loadModules, async (input, { preprocess, diagram }) => {
  const processed = preprocess.preprocessDiagram(input.text);
  const parsed = await diagram.Diagram.fromText(processed.code, { title: processed.title });
  if (!railroadTypes.includes(parsed.type)) {
    throw new Error(`detected as ${parsed.type}, not a railroad diagram`);
  }
  const db = parsed.db;
  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    rules: db.getRules().map((rule) => ({
      name: rule.name,
      definition: expression(rule.definition),
    })),
  };
});
