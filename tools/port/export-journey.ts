/**
 * Canonical JSON of user journey diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The parser fills the journey DB through its
 * own calls, and the DB's sanitising of the header texts runs DOMPurify, so the reference runs
 * with a window (`provideDomForSanitising`). The DB holds the entities as placeholders; the
 * canonical text is the text the reader sees, which the renderer reaches through `.text(…)`, the
 * serializer's escaping, and `decodeEntities` on the SVG string, read back by the HTML parser.
 * `displayText` does the same with the reference's own `decodeEntities` and a DOM. A score that is
 * not finite is written as its sentinel string, as `value.rs` writes it.
 *
 * Usage: `tsx tools/port/export-journey.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface JourneyTask {
  section: string;
  task: string;
  score: number;
  people: string[];
}

interface JourneyDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getSections(): string[];
  getTasks(): JourneyTask[];
  getActors(): string[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: {
    encodeEntities(text: string): string;
    decodeEntities(text: string): string;
  };
  /** The Jison parser of `journey.jison`, whose `yy` is the DB. */
  grammar: { yy: unknown; parse(text: string): unknown };
  db: { default: JourneyDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, grammar, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/user-journey/parser/journey.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/user-journey/journeyDb.js'),
  ]);
  return { preprocess, utils, grammar: grammar.parser, db };
}

/** The text a reader sees for a DB text: escaped as the serializer does, then decoded. */
function displayText(utils: Modules['utils'], text: string): string {
  const escaped = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const element = document.createElement('div');
  element.innerHTML = utils.decodeEntities(escaped);
  return element.textContent ?? '';
}

/** A score as JSON: the number when it is finite, else `"NaN"`, `"Infinity"` or `"-Infinity"`. */
const score = (value: number): number | string => (Number.isFinite(value) ? value : String(value));

await runExporter(loadModules, (input, { preprocess, utils, grammar, db }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const journeyDb = db.default;
  journeyDb.clear();
  grammar.yy = journeyDb;
  // `Diagram.fromText` sets the front matter title before the parse.
  if (title) {
    journeyDb.setDiagramTitle(title);
  }
  grammar.parse(utils.encodeEntities(code.cleaned) + '\n');

  // The renderer reads the tasks first, and `getActors` reads the tasks it has compiled.
  const tasks = journeyDb.getTasks();
  const actors = journeyDb.getActors();
  const show = (text: string) => displayText(utils, text);

  return {
    version: 1,
    title: show(journeyDb.getDiagramTitle()),
    accTitle: show(journeyDb.getAccTitle()),
    accDescr: show(journeyDb.getAccDescription()),
    sections: journeyDb.getSections().map(show),
    tasks: tasks.map((task) => ({
      section: show(task.section),
      task: show(task.task),
      score: score(task.score),
      people: task.people.map(show),
    })),
    actors: actors.map(show),
  };
});
