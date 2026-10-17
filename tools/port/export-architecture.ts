/**
 * Canonical JSON of architecture diagrams, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText`: the text is detected, the DB is cleared, the front
 * matter title is set, and the text (with `encodeEntities` and a line break) goes to the Langium
 * parser of `architectureParser.ts`, which fills the architecture DB in its own order (groups, then
 * services, junctions, edges and alignments). The model is read out of that DB, and its derived
 * block comes from `getDataStructures()`. The DB's header setters sanitise with DOMPurify, which
 * needs a window.
 *
 * Usage: `tsx tools/port/export-architecture.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface ArchitectureDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getGroups(): { id: string; icon?: string; title?: string; in?: string }[];
  getServices(): { id: string; icon?: string; iconText?: string; title?: string; in?: string }[];
  getJunctions(): { id: string; in?: string }[];
  getEdges(): {
    lhsId: string;
    lhsDir: string;
    lhsInto?: boolean;
    lhsGroup?: boolean;
    rhsId: string;
    rhsDir: string;
    rhsInto?: boolean;
    rhsGroup?: boolean;
    title?: string;
  }[];
  getLayoutHints(): { direction: string; members: string[] }[];
  getDataStructures(): {
    groupAlignments: Map<string, string>;
    spatialMaps: Map<string, [number, number]>[];
  };
}

interface ArchitectureParser {
  parser: { yy?: unknown };
  parse(text: string): Promise<void>;
}

interface Modules {
  detectType: { detectType(text: string): string };
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  parser: ArchitectureParser;
  ArchitectureDB: new () => ArchitectureDb;
}

interface CanonicalModel {
  version: number;
  title: string;
  accTitle: string;
  accDescr: string;
  groups: { id: string; icon: string; title: string; in: string }[];
  services: { id: string; icon: string; iconText: string; title: string; in: string }[];
  junctions: { id: string; in: string }[];
  edges: {
    lhsId: string;
    lhsDir: string;
    lhsInto: boolean;
    lhsGroup: boolean;
    rhsId: string;
    rhsDir: string;
    rhsInto: boolean;
    rhsGroup: boolean;
    title: string;
  }[];
  alignments: { direction: string; members: string[] }[];
  derived: {
    groupAlignments: [string, string][];
    spatialMaps: { id: string; x: number; y: number }[][];
  };
}

async function loadModules(reference: Reference): Promise<Modules> {
  // DOMPurify takes its window when its module loads, so the window comes first.
  provideDomForSanitising();
  await registerDetectors(reference);
  const [detectType, preprocess, encode, parser, db] = await Promise.all([
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: ArchitectureParser }>(
      '/packages/mermaid/src/diagrams/architecture/architectureParser.ts'
    ),
    reference.load<{ ArchitectureDB: Modules['ArchitectureDB'] }>(
      '/packages/mermaid/src/diagrams/architecture/architectureDb.ts'
    ),
  ]);
  return {
    detectType,
    preprocess,
    encode,
    parser: parser.parser,
    ArchitectureDB: db.ArchitectureDB,
  };
}

async function convert(input: { text: string }, modules: Modules): Promise<CanonicalModel> {
  const { detectType, preprocess, encode, parser, ArchitectureDB } = modules;

  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned);
  } catch {
    diagramType = undefined;
  }
  if (diagramType !== 'architecture') {
    throw new Error(`not an architecture diagram (detected: ${diagramType ?? 'none'})`);
  }

  // `Diagram.fromText`: the DB is a new instance, cleared, and given the front matter title.
  const db = new ArchitectureDB();
  parser.parser.yy = db;
  db.clear();
  if (processed.title) {
    db.setDiagramTitle(processed.title);
  }
  const text = encode.encodeEntities(processed.code.cleaned) + '\n';
  await parser.parse(text);

  const structures = db.getDataStructures();
  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    groups: db.getGroups().map((group) => ({
      id: group.id,
      icon: group.icon ?? '',
      title: group.title ?? '',
      in: group.in ?? '',
    })),
    services: db.getServices().map((service) => ({
      id: service.id,
      icon: service.icon ?? '',
      iconText: service.iconText ?? '',
      title: service.title ?? '',
      in: service.in ?? '',
    })),
    junctions: db.getJunctions().map((junction) => ({
      id: junction.id,
      in: junction.in ?? '',
    })),
    edges: db.getEdges().map((edge) => ({
      lhsId: edge.lhsId,
      lhsDir: edge.lhsDir,
      lhsInto: edge.lhsInto === true,
      lhsGroup: edge.lhsGroup === true,
      rhsId: edge.rhsId,
      rhsDir: edge.rhsDir,
      rhsInto: edge.rhsInto === true,
      rhsGroup: edge.rhsGroup === true,
      title: edge.title ?? '',
    })),
    alignments: db.getLayoutHints().map((hint) => ({
      direction: hint.direction,
      members: [...hint.members],
    })),
    derived: {
      groupAlignments: [...structures.groupAlignments],
      spatialMaps: structures.spatialMaps.map((spatialMap) =>
        [...spatialMap].map(([id, [x, y]]) => ({ id, x, y }))
      ),
    },
  };
}

await runExporter(loadModules, convert);
