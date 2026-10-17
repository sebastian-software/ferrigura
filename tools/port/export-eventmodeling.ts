/**
 * Canonical JSON of event modeling diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram` and `encodeEntities` as `Diagram.fromText` gives it to
 * the parser: the parser reads the text with its comment lines (`preserveCommentsWhenParsing`)
 * unless that is longer than `maxTextSize`. The Langium parser of `@mermaid-js/parser` returns the
 * AST, and the DB takes it as `parser.ts` does, with the common DB's header setters, which
 * sanitise with DOMPurify and so need a window (`provideDomForSanitising`).
 *
 * The DB sizes its boxes with `getBBox`, which jsdom does not have. The sizes never reach the
 * model, so the exporter gives `getBBox` the fixed box of the diagram's own spec. The frames' lanes
 * and namespaces are the DB's, and the namespace is read here by the rule of `extractNamespace`,
 * which the DB does not export.
 *
 * Usage: `tsx tools/port/export-eventmodeling.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** `maxTextSize`, the default configuration's, in UTF-16 units as the reference counts it. */
const MAX_TEXT_SIZE = 50000;

interface EmReference {
  $refText: string;
}

interface EmFrame {
  $type: 'EmTimeFrame' | 'EmResetFrame';
  name: string;
  modelEntityType: string;
  entityIdentifier: string;
  sourceFrames: EmReference[];
  dataInlineValue?: string;
  dataReference?: EmReference;
}

interface EmDataEntity {
  name: string;
  dataBlockValue: string;
}

interface EventModel {
  frames: EmFrame[];
  dataEntities: EmDataEntity[];
}

interface Swimlane {
  index: number;
  label: string;
}

interface Box {
  frame: EmFrame;
  index: number;
  swimlane: Swimlane;
}

interface Relation {
  sourceBox: Box;
  targetBox: Box;
}

interface EventModelingDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  setAst(ast: EventModel): void;
  getState(): { boxes: Box[]; relations: Relation[]; sortedSwimlanesArray: Swimlane[] };
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string; withComments?: string };
      title?: string;
    };
  };
  utils: { encodeEntities(text: string): string };
  parser: { parse(diagramType: 'eventmodeling', text: string): Promise<EventModel> };
  common: {
    populateCommonDb(ast: EventModel, db: EventModelingDb): void;
    sanitizeText(text: string, config: unknown): string;
  };
  config: { getConfig(): unknown };
  db: { db: EventModelingDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, parser, populate, sanitise, config, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['parser']>('/packages/parser/src/parse.ts'),
    reference.load<{ populateCommonDb: Modules['common']['populateCommonDb'] }>(
      '/packages/mermaid/src/diagrams/common/populateCommonDb.ts'
    ),
    reference.load<Pick<Modules['common'], 'sanitizeText'>>(
      '/packages/mermaid/src/diagrams/common/common.ts'
    ),
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/eventmodeling/db.ts'),
  ]);
  stubTextMeasurement();
  const common = {
    populateCommonDb: populate.populateCommonDb,
    sanitizeText: sanitise.sanitizeText,
  };
  return { preprocess, utils, parser, common, config, db };
}

/** Gives text elements the fixed box of the diagram's own spec, which jsdom cannot measure. */
function stubTextMeasurement(): void {
  const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
  Object.defineProperty(Object.getPrototypeOf(text), 'getBBox', {
    configurable: true,
    value: () => ({ x: 0, y: 0, width: 100, height: 20 }),
  });
}

/**
 * The data text of a frame before its line breaks (`calculateTextProps`, `db.ts:349-372`). The DB
 * does not export the function, so the two cuts are transcribed from it, and the sanitiser is the
 * reference's own.
 */
function dataText(
  frame: EmFrame,
  dataEntities: EmDataEntity[],
  sanitize: (text: string) => string
): string | null {
  let toHtml: string | undefined;
  if (frame.dataInlineValue) {
    toHtml = frame.dataInlineValue.substring(frame.dataInlineValue.indexOf('{') + 1);
    toHtml = toHtml.substring(0, toHtml.lastIndexOf('}') - 1);
    toHtml = sanitize(toHtml);
  }
  if (frame.dataReference) {
    const dataEntity = dataEntities.find(
      (candidate) => candidate.name === frame.dataReference?.$refText
    );
    if (dataEntity) {
      toHtml = dataEntity.dataBlockValue;
      toHtml = toHtml.substring(toHtml.indexOf('{\n') + 2);
      toHtml = toHtml.substring(0, toHtml.lastIndexOf('}') - 1);
      toHtml = sanitize(toHtml);
    }
  }
  return toHtml ?? null;
}

/** The model of a frame, as the DB holds it: its entity, its lane and the data it names. */
function frameModel(frame: EmFrame, lane: number, data: string | null) {
  const parts = frame.entityIdentifier.split('.');
  const hasNamespace = parts.length === 2;
  return {
    name: frame.name,
    kind: frame.$type === 'EmResetFrame' ? 'reset' : 'time',
    modelEntityType: frame.modelEntityType,
    entityIdentifier: frame.entityIdentifier,
    entityName: hasNamespace ? parts[1] : frame.entityIdentifier,
    namespace: hasNamespace ? parts[0] : null,
    lane,
    sourceFrames: frame.sourceFrames.map((reference) => reference.$refText),
    dataInline: frame.dataInlineValue ?? null,
    dataReference: frame.dataReference?.$refText ?? null,
    data,
  };
}

await runExporter(loadModules, async (input, { preprocess, utils, parser, common, config, db }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  // `Diagram.fromText`: the text with comments is parsed unless it is too long.
  const withComments = code.withComments;
  const source = withComments && withComments.length <= MAX_TEXT_SIZE ? withComments : code.cleaned;
  const text = utils.encodeEntities(source) + '\n';

  db.db.clear();
  if (title) {
    db.db.setDiagramTitle(title);
  }
  // `parser.ts`: the AST goes to the DB, then the common header setters run.
  const ast = await parser.parse('eventmodeling', text);
  db.db.setAst(ast);
  common.populateCommonDb(ast, db.db);

  const state = db.db.getState();
  const sanitize = (value: string) => common.sanitizeText(value, config.getConfig());
  return {
    version: 1,
    title: db.db.getDiagramTitle(),
    accTitle: db.db.getAccTitle(),
    accDescr: db.db.getAccDescription(),
    frames: state.boxes.map((box) =>
      frameModel(box.frame, box.swimlane.index, dataText(box.frame, ast.dataEntities, sanitize))
    ),
    dataEntities: ast.dataEntities.map((data) => ({
      name: data.name,
      dataBlockValue: data.dataBlockValue,
    })),
    lanes: state.sortedSwimlanesArray.map((lane) => ({ index: lane.index, label: lane.label })),
    relations: state.relations.map((relation) => ({
      sourceFrame: relation.sourceBox.index,
      targetFrame: relation.targetBox.index,
    })),
  };
});
