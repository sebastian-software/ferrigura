/**
 * Canonical JSON of packet diagrams, from the TypeScript reference.
 *
 * The configuration of the input is set as `mermaidAPI` sets it, because the DB reads `bitsPerRow`
 * during the parse. Then the steps of `Diagram.fromText` run: the DB is the one the parser writes
 * to (`parser.parser.yy`), it is cleared, the front matter title is set, and the parser reads the
 * text with `encodeEntities` and a line break. The model is the DB state after the parse. The
 * DB's sanitising runs DOMPurify, so the reference runs with a window (`provideDomForSanitising`).
 * A bit number that is not finite is written as its sentinel string, as `packet.rs` writes it.
 *
 * Usage: `tsx tools/port/export-packet.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

interface PacketBlock {
  start: number;
  end: number;
  bits: number;
  label: string;
}

interface PacketDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getPacket(): PacketBlock[][];
}

interface Modules {
  config: {
    setSiteConfig(conf: Record<string, unknown>): unknown;
    reset(): void;
    getConfig(): unknown;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Record<string, unknown>): void;
  };
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string };
      title?: string;
      config: Record<string, unknown>;
    };
  };
  detectType: {
    detectType(text: string, config?: unknown): string;
  };
  utils: { encodeEntities(text: string): string };
  /** The `ParserDefinition` of `parser.ts`: its `parse` reads the DB from `parser.parser.yy`. */
  parser: {
    parser: { yy: unknown };
    parse(text: string): Promise<void>;
  };
  db: { PacketDB: new () => PacketDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, preprocess, detectType, utils, parser, db] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['utils']>('/packages/mermaid/src/utils.ts'),
    reference.load<{ parser: Modules['parser'] }>(
      '/packages/mermaid/src/diagrams/packet/parser.ts'
    ),
    reference.load<Modules['db']>('/packages/mermaid/src/diagrams/packet/db.ts'),
  ]);
  return { config, preprocess, detectType, utils, parser: parser.parser, db };
}

/** A bit number as JSON: the number when it is finite, else the sentinel string. */
const bitNumber = (value: number): number | string =>
  Number.isFinite(value) ? value : String(value);

await runExporter(
  loadModules,
  async (input, { config, preprocess, detectType, utils, parser, db }) => {
    // The configuration of the input alone, as a render gives it: the site settings, the reset, the
    // detected scope, then the front matter and directives (`processAndSetConfigs`).
    config.setSiteConfig({});
    config.reset();
    const processed = preprocess.preprocessDiagram(input.text);
    let diagramType: string | undefined;
    try {
      diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
    } catch {
      diagramType = undefined;
    }
    config.setDiagramConfigScope(diagramType);
    config.addDirective(processed.config);

    const packetDb = new db.PacketDB();
    parser.parser.yy = packetDb;
    packetDb.clear();
    if (processed.title) {
      packetDb.setDiagramTitle(processed.title);
    }
    await parser.parse(utils.encodeEntities(processed.code.cleaned) + '\n');

    return {
      version: 1,
      title: packetDb.getDiagramTitle(),
      accTitle: packetDb.getAccTitle(),
      accDescr: packetDb.getAccDescription(),
      rows: packetDb.getPacket().map((word) =>
        word.map((block) => ({
          start: bitNumber(block.start),
          end: bitNumber(block.end),
          bits: bitNumber(block.bits),
          label: block.label,
        }))
      ),
    };
  }
);
