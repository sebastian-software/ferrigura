/**
 * Canonical JSON of timeline diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then through the type check of `detectType` (a text
 * that is not detected as a timeline is rejected, as `Diagram.fromText` rejects it), then through
 * `encodeEntities` and a line break, as `Diagram.fromText` gives it to the parser. The Jison parser
 * fills the timeline DB, and the DB's state is read out as the canonical model. The header texts
 * are kept by the common DB, whose setters sanitise them with DOMPurify, so the reference runs with
 * a window (`provideDomForSanitising`). The front matter title is dropped: the timeline DB has no
 * setter for it. See `docs-port/timeline/PLAN.md`.
 *
 * Usage: `tsx tools/port/export-timeline.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface TimelineTask {
  section: string;
  task: string;
  events: string[];
}

interface TimelineDb {
  clear(): void;
  getDirection(): string;
  getSections(): string[];
  getTasks(): TimelineTask[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string } };
  };
  encode: { encodeEntities(text: string): string };
  detect: { detectType(text: string, config?: unknown): string };
  config: { getConfig(): unknown };
  parser: { yy?: unknown; parse(text: string): unknown };
  db: TimelineDb;
  common: {
    getDiagramTitle(): string;
    getAccTitle(): string;
    getAccDescription(): string;
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, encode, detect, config, jison, db, common] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['detect']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<{ parser: Modules['parser'] }>(
      '/packages/mermaid/src/diagrams/timeline/parser/timeline.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/timeline/timelineDb.js'),
    reference.load<Modules['common']>('/packages/mermaid/src/diagrams/common/commonDb.ts'),
  ]);
  return { preprocess, encode, detect, config, parser: jison.parser, db, common };
}

await runExporter(loadModules, (input, modules) => {
  const { preprocess, encode, detect, config, parser, db, common } = modules;
  const { code } = preprocess.preprocessDiagram(input.text);
  if (detect.detectType(code.cleaned, config.getConfig()) !== 'timeline') {
    throw new Error('the text is not detected as a timeline');
  }

  // `Diagram.fromText`: the DB is cleared and becomes the parser's `yy` before the parse.
  db.clear();
  parser.yy = db;
  parser.parse(encode.encodeEntities(code.cleaned) + '\n');

  // The arrays are copied: the DB reuses them on the next `clear`.
  return {
    version: 1,
    direction: db.getDirection(),
    title: common.getDiagramTitle(),
    accTitle: common.getAccTitle(),
    accDescr: common.getAccDescription(),
    sections: [...db.getSections()],
    periods: db.getTasks().map((task) => ({
      section: task.section,
      period: task.task,
      events: [...task.events],
    })),
  };
});
