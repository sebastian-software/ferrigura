/**
 * Canonical JSON of ER diagrams, from the TypeScript reference.
 *
 * The text goes through `preprocessDiagram`, then `encodeEntities` and a line break, as
 * `Diagram.fromText` gives it to the parser. The Jison parser fills the ER DB, which is used as it
 * is: its header setters, the subgraph titles and the front-matter title are sanitised with
 * DOMPurify, so the reference runs with a window (`provideDomForSanitising`). The DB's state is
 * read out as the canonical model (`docs-port/er/PLAN.md`, sections 5 and 6).
 *
 * Usage: `tsx tools/port/export-er.ts inputs.json outputs.json`
 */
import {
  provideDomForSanitising,
  registerDetectors,
  registerErDetector,
  type Reference,
} from './reference.ts';
import { runExporter } from './exporter.ts';

interface ErAttributeDb {
  type: string;
  name: string;
  keys: string[];
  comment: string;
}

interface ErDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getDirection(): string;
  getEntities(): Map<
    string,
    {
      id: string;
      alias: string;
      attributes: ErAttributeDb[];
      cssClasses?: string;
      cssStyles?: string[];
    }
  >;
  getRelationships(): {
    entityA: string;
    roleA: string;
    entityB: string;
    relSpec: { cardA: string; relType: string; cardB: string };
  }[];
  getClasses(): Map<string, { id: string; styles: string[]; textStyles: string[] }>;
  getSubGraphs(): {
    id: string;
    title: string;
    nodes: string[];
    classes: string[];
    cssStyles: string[];
    dir?: string;
  }[];
}

interface Modules {
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  erParser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
  erDb: { ErDB: new () => ErDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  await registerErDetector(reference);

  const [preprocess, encode, erParser, erDb] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['erParser']>('/packages/mermaid/src/diagrams/er/parser/erDiagram.jison'),
    reference.load<Modules['erDb']>('/packages/mermaid/src/diagrams/er/erDb.ts'),
  ]);
  return { preprocess, encode, erParser, erDb };
}

await runExporter(loadModules, (input, modules) => {
  const { preprocess, encode, erParser, erDb } = modules;
  const { code, title } = preprocess.preprocessDiagram(input.text);

  const db = new erDb.ErDB();
  db.clear();
  // `Diagram.fromText` sets the front-matter title after `clear`, and before the parser runs.
  if (title) {
    db.setDiagramTitle(title);
  }
  erParser.default.parser.yy = db;
  erParser.default.parse(encode.encodeEntities(code.cleaned) + '\n');

  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    direction: db.getDirection(),
    entities: [...db.getEntities()].map(([name, entity]) => ({
      id: entity.id,
      name,
      alias: entity.alias,
      attributes: entity.attributes.map((attribute) => ({
        type: attribute.type,
        name: attribute.name,
        keys: attribute.keys,
        comment: attribute.comment,
      })),
      // The DB starts every entity with the class `default`, which the model leaves out.
      classes: (entity.cssClasses ?? '').split(' ').slice(1),
      styles: entity.cssStyles ?? [],
    })),
    relationships: db.getRelationships().map((relationship) => ({
      entityA: relationship.entityA,
      roleA: relationship.roleA,
      entityB: relationship.entityB,
      relSpec: {
        cardA: relationship.relSpec.cardA,
        relType: relationship.relSpec.relType,
        cardB: relationship.relSpec.cardB,
      },
    })),
    classes: [...db.getClasses().values()].map((cls) => ({
      id: cls.id,
      styles: cls.styles,
      textStyles: cls.textStyles,
    })),
    subgraphs: db.getSubGraphs().map((subgraph) => ({
      id: subgraph.id,
      title: subgraph.title,
      nodes: subgraph.nodes,
      classes: subgraph.classes,
      styles: subgraph.cssStyles,
      dir: subgraph.dir ?? null,
    })),
  };
});
