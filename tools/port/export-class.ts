/**
 * Canonical JSON of class diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram` (front matter, directives, comments), and the front
 * matter's title goes to `setDiagramTitle` as `Diagram.ts` does. The class Jison parser then fills
 * the class DB, whose header setters are the real `commonDb` setters (sanitising with DOMPurify,
 * which needs the DOM that `provideDomForSanitising` gives it). The DB's state is read out as the
 * canonical model; the enum-like values (relation markers, line types) take the names of
 * `docs-port/class/PLAN.md`, section 5. See `docs-port/class/PLAN.md`.
 *
 * Usage: `tsx tools/port/export-class.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface ClassMemberJs {
  id: string;
  memberType: 'method' | 'attribute';
  visibility: string;
  classifier: string;
  parameters?: string;
  returnType?: string;
  text: string;
}

interface ClassNodeJs {
  id: string;
  type: string;
  label: string;
  text: string;
  cssClasses: string;
  annotations: string[];
  members: ClassMemberJs[];
  methods: ClassMemberJs[];
  styles: string[];
  parent?: string;
  link?: string;
  linkTarget?: string;
  tooltip?: string;
  haveCallback?: boolean;
}

interface ClassRelationJs {
  id1: string;
  id2: string;
  relationTitle1: string;
  relationTitle2: string;
  title?: string;
  relation: { type1: number | 'none'; type2: number | 'none'; lineType: number };
}

interface ClassNoteJs {
  id: string;
  class?: string;
  text: string;
  index: number;
  parent?: string;
}

interface NamespaceJs {
  id: string;
  label: string;
  parent?: string;
  explicit: boolean;
  classes: Map<string, unknown>;
  notes: Map<string, unknown>;
}

interface ClassDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getDirection(): string;
  getClasses(): Map<string, ClassNodeJs>;
  getRelations(): ClassRelationJs[];
  getNotes(): Map<string, ClassNoteJs>;
  getNamespaces(): Map<string, NamespaceJs>;
  styleClasses: Map<string, { id: string; styles: string[] }>;
  interfaces: { id: string; label: string; classId: string }[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  parser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  classDb: { ClassDB: new () => ClassDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, parser, classDb] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['parser']>(
      '/packages/mermaid/src/diagrams/class/parser/classDiagram.jison'
    ),
    reference.load<Modules['classDb']>('/packages/mermaid/src/diagrams/class/classDb.ts'),
  ]);
  return { preprocess, parser, classDb };
}

/** The name of a relation end, by `getArrowMarker` of the DB; `none` for the sentinel. */
const markerName = (value: number | 'none'): string => {
  const names = ['aggregation', 'extension', 'composition', 'dependency', 'lollipop'];
  return typeof value === 'number' ? (names[value] ?? 'none') : 'none';
};

const memberOf = (member: ClassMemberJs) => ({
  memberType: member.memberType,
  visibility: member.visibility,
  classifier: member.classifier,
  id: member.id,
  parameters: member.parameters ?? null,
  returnType: member.returnType ?? null,
  text: member.text,
});

await runExporter(loadModules, (input, { preprocess, parser, classDb }) => {
  const { code, title } = preprocess.preprocessDiagram(input.text);
  const db = new classDb.ClassDB();
  db.clear();
  if (title) {
    db.setDiagramTitle(title);
  }
  parser.default.parser.yy = db;
  parser.default.parse(code.cleaned);

  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    direction: db.getDirection(),
    classes: [...db.getClasses().values()].map((cls) => ({
      id: cls.id,
      type: cls.type,
      label: cls.label,
      text: cls.text,
      cssClasses: cls.cssClasses,
      annotations: cls.annotations,
      members: cls.members.map(memberOf),
      methods: cls.methods.map(memberOf),
      styles: cls.styles,
      parent: cls.parent ?? null,
      link: cls.link ?? null,
      linkTarget: cls.linkTarget ?? null,
      tooltip: cls.tooltip ?? null,
      haveCallback: cls.haveCallback ?? null,
    })),
    relations: db.getRelations().map((relation) => ({
      id1: relation.id1,
      id2: relation.id2,
      relationTitle1: relation.relationTitle1,
      relationTitle2: relation.relationTitle2,
      title: relation.title ?? null,
      type1: markerName(relation.relation.type1),
      type2: markerName(relation.relation.type2),
      lineType: relation.relation.lineType === 1 ? 'dotted' : 'solid',
    })),
    notes: [...db.getNotes().values()].map((note) => ({
      id: note.id,
      class: note.class ?? null,
      text: note.text,
      index: note.index,
      parent: note.parent ?? null,
    })),
    interfaces: db.interfaces.map((item) => ({
      id: item.id,
      label: item.label,
      classId: item.classId,
    })),
    namespaces: [...db.getNamespaces().values()].map((namespace) => ({
      id: namespace.id,
      label: namespace.label,
      parent: namespace.parent ?? null,
      explicit: namespace.explicit,
      classes: [...namespace.classes.keys()],
      notes: [...namespace.notes.keys()],
    })),
    styleClasses: [...db.styleClasses.values()].map((style) => ({
      id: style.id,
      styles: style.styles,
    })),
  };
});
