/**
 * Canonical JSON of requirement diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the Jison parser. The reference creates a new DB for each diagram
 * (`get db()` in `requirementDiagram.ts`), so each input gets a fresh `RequirementDB`. The model is
 * the DB's state after the parse, with the texts as the DB holds them: the entity placeholders stay,
 * and the header texts are the sanitised ones (`docs-port/requirement/PLAN.md`, section 5). The
 * header texts go through DOMPurify, so the reference runs with a window (`provideDomForSanitising`).
 *
 * Usage: `tsx tools/port/export-requirement.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface ReferenceRequirement {
  name: string;
  type: string;
  requirementId: string;
  text: string;
  risk: string;
  verifyMethod: string;
  classes: string[];
  cssStyles: string[];
}

interface ReferenceElement {
  name: string;
  type: string;
  docRef: string;
  classes: string[];
  cssStyles: string[];
}

interface ReferenceRelation {
  type: string;
  src: string;
  dst: string;
}

interface ReferenceClass {
  id: string;
  styles: string[];
  textStyles: string[];
}

interface RequirementDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDirection(): string;
  getRequirements(): Map<string, ReferenceRequirement>;
  getElements(): Map<string, ReferenceElement>;
  getRelationships(): ReferenceRelation[];
  getClasses(): Map<string, ReferenceClass>;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  utils: {
    encodeEntities(text: string): string;
  };
  /** The Jison parser of `requirementDiagram.jison`, whose `yy` is the DB. */
  grammar: { yy: unknown; parse(text: string): unknown };
  db: { RequirementDB: new () => RequirementDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, utils, grammar, db] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['grammar'] }>(
      '/packages/mermaid/src/diagrams/requirement/parser/requirementDiagram.jison'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/requirement/requirementDb.ts'),
  ]);
  return { preprocess, utils, grammar: grammar.parser, db };
}

await runExporter(loadModules, (input, { preprocess, utils, grammar, db }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const requirementDb = new db.RequirementDB();
  requirementDb.clear();
  grammar.yy = requirementDb;
  // `Diagram.fromText` sets the front matter title before the parse.
  if (title) {
    requirementDb.setDiagramTitle(title);
  }
  grammar.parse(utils.encodeEntities(code.cleaned) + '\n');

  return {
    version: 1,
    direction: requirementDb.getDirection(),
    requirements: [...requirementDb.getRequirements().values()].map((requirement) => ({
      name: requirement.name,
      type: requirement.type,
      requirementId: requirement.requirementId,
      text: requirement.text,
      risk: requirement.risk,
      verifyMethod: requirement.verifyMethod,
      classes: requirement.classes,
      cssStyles: requirement.cssStyles,
    })),
    elements: [...requirementDb.getElements().values()].map((element) => ({
      name: element.name,
      type: element.type,
      docRef: element.docRef,
      classes: element.classes,
      cssStyles: element.cssStyles,
    })),
    relationships: requirementDb.getRelationships().map((relation) => ({
      type: relation.type,
      src: relation.src,
      dst: relation.dst,
    })),
    classes: [...requirementDb.getClasses().values()].map((styleClass) => ({
      id: styleClass.id,
      styles: styleClass.styles,
      textStyles: styleClass.textStyles,
    })),
    title: requirementDb.getDiagramTitle(),
    accTitle: requirementDb.getAccTitle(),
    accDescr: requirementDb.getAccDescription(),
  };
});
