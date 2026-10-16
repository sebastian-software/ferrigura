/**
 * Canonical JSON of sequence diagrams, from the TypeScript reference.
 *
 * Each text goes through the steps `mermaidAPI.parse` and `Diagram.fromText` run
 * (`mermaidAPI.ts:72-87`, `Diagram.ts:19-83`): `preprocessDiagram`, then the configuration of the
 * input (reset, the scope of the detected type, the directives), then the DB's `clear`, the
 * diagram's `init` with that configuration, the front matter title through `setDiagramTitle`, and
 * the Jison parser on `encodeEntities(cleaned) + "\n"`. The DB state is read out as the canonical
 * model, `docs-port/sequence/PLAN.md` section 7.
 *
 * The configuration is pinned after the directives to `htmlLabels` true and `securityLevel`
 * strict, the reference's defaults, so that `sanitizeMore` takes its DOMPurify path (`common.ts`).
 * A directive could set either key otherwise (`configKeys` has both).
 *
 * The box colour check (`sequenceDb.ts:400-404`) calls `window.CSS.supports('color', value)` when
 * `window.CSS` exists, and falls back to jsdom's style parser when it does not. The window gets
 * `cssSupports`, which answers as Chromium does for the colours a box can name (see there).
 *
 * Usage: `tsx tools/port/export-sequence.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type Config = Record<string, unknown>;

/** The actor record of `sequenceDb.ts` (`addActor`), without the render-only fields. */
interface ReferenceActor {
  box?: ReferenceBox;
  name: string;
  description: string;
  wrap: boolean;
  prevActor?: string;
  nextActor?: string;
  links: Config;
  properties: Config;
  type: unknown;
}

interface ReferenceBox {
  name?: string;
  wrap: boolean;
  fill: string;
  actorKeys: string[];
}

interface ReferenceMessage {
  id: string;
  from?: string;
  to?: string;
  message: string | { start?: number; step?: number; visible: boolean };
  wrap: boolean;
  type: number;
  activate?: boolean;
  centralConnection?: number;
  placement?: number;
}

interface SequenceDb {
  readonly LINETYPE: Record<string, number>;
  clear(): void;
  setDiagramTitle(text: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getActors(): Map<string, ReferenceActor>;
  getBoxes(): ReferenceBox[];
  getMessages(): ReferenceMessage[];
  getCreatedActors(): Map<string, number>;
  getDestroyedActors(): Map<string, number>;
}

interface SequenceDefinition {
  /** A getter in the reference: each read makes a new DB. */
  readonly db: SequenceDb;
  init: (config: Config) => void;
  parser: { parser: { yy: unknown }; parse(text: string): unknown };
}

interface Modules {
  config: {
    reset(): void;
    getConfig(): Config;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Config): void;
    setConfig(conf: Config): unknown;
  };
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string };
      title?: string;
      config?: Config;
    };
  };
  detectType: { detectType(text: string, config?: unknown): string };
  encode: { encodeEntities(text: string): string };
  sequence: { diagram: SequenceDefinition };
}

/**
 * The CSS named colours (CSS Color 4), as the css-color package lists them. The cssstyle package,
 * which jsdom uses for its colours, takes its names from that package.
 */
const NAMED_COLORS = new Set(
  `
  aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet
  brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan
  darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki darkmagenta
  darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen darkslateblue
  darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey
  dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray
  green greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender lavenderblush
  lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen
  lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey
  lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue
  mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise
  mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive
  olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred
  papayawhip peachpuff peru pink plum powderblue purple rebeccapurple red rosybrown royalblue
  saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue slateblue slategray
  slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white
  whitesmoke yellow yellowgreen
  `
    .split(/\s+/)
    .filter(Boolean)
);

/** `transparent`, `currentcolor` and the CSS-wide keywords, which every property accepts. */
const KEYWORDS = new Set([
  'transparent',
  'currentcolor',
  'inherit',
  'initial',
  'unset',
  'revert',
  'revert-layer',
]);

// The functional colours. A number is the CSS number (no trailing dot); a percentage is a number
// with `%`; a hue may carry an angle unit. The legacy comma form and the modern space form are
// both accepted, each with uniform argument kinds (`rgb(1%, 2, 3)` is not), and the alpha is an
// optional fourth argument. Anything else, such as `hsl(120, 100, 50)`, is answered false: this is
// the conservative part of the rule, and the port copies it.
const number = String.raw`[+-]?(?:\d+(?:\.\d+)?|\.\d+)(?:e[+-]?\d+)?`;
const percent = `${number}%`;
const hue = `${number}(?:deg|grad|rad|turn)?`;
const alpha = `(?:${number}|${percent})`;
const comma = String.raw`\s*,\s*`;
const space = String.raw`\s+`;
const slash = String.raw`\s*/\s*`;
const rgbArguments = [
  `(?:${number}${comma}${number}${comma}${number}|${percent}${comma}${percent}${comma}${percent})(?:${comma}${alpha})?`,
  `(?:${number}${space}${number}${space}${number}|${percent}${space}${percent}${space}${percent})(?:${slash}${alpha})?`,
];
const hslArguments = [
  `${hue}${comma}${percent}${comma}${percent}(?:${comma}${alpha})?`,
  `${hue}${space}${percent}${space}${percent}(?:${slash}${alpha})?`,
];
const colorFunction = new RegExp(
  String.raw`^(?:rgba?\(\s*(?:${rgbArguments.join('|')})\s*\)|hsla?\(\s*(?:${hslArguments.join('|')})\s*\))$`,
  'i'
);

/**
 * `CSS.supports('color', value)` as Chromium answers it for the values a box colour can take.
 * The reference passes `\w*` or one of the lower-case `rgb`, `rgba`, `hsl`, `hsla` functions
 * (`sequenceDb.ts:395`), so the keywords are the named colours (ASCII case-insensitive, as in
 * Chromium), `transparent`, `currentcolor` and the CSS-wide keywords, which Chromium accepts for any
 * property (`inherit` and the others; `\w*` can reach the first four), and the functions above.
 * Other properties are false. The CSS system colours (`Canvas`, `ButtonFace`, ...) are not in the
 * rule: not checked against Chromium yet.
 */
function cssSupports(property: string, value: string): boolean {
  if (property !== 'color') return false;
  // ASCII case only, as Chromium matches keywords: `toLowerCase` would fold `K` (Kelvin sign) too.
  const keyword = value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());
  return NAMED_COLORS.has(keyword) || KEYWORDS.has(keyword) || colorFunction.test(value);
}

/** Gives the reference's window the `CSS.supports` that `parseBoxData` checks for. */
function provideCssSupports(): void {
  const win = (globalThis as unknown as { window: object }).window;
  Object.defineProperty(win, 'CSS', { value: { supports: cssSupports }, configurable: true });
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  provideCssSupports();
  await registerDetectors(reference);
  const [config, preprocess, detectType, encode, sequence] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['sequence']>(
      '/packages/mermaid/src/diagrams/sequence/sequenceDiagram.ts'
    ),
  ]);
  return { config, preprocess, detectType, encode, sequence };
}

/** The reference's `undefined` as `null`; every other value as it is. */
function nullable<T>(value: T | undefined): T | null {
  return value ?? null;
}

/** The name of each `LINETYPE` value, as the model writes a line type. */
function lineTypeName(names: Map<number, string>, value: number): string {
  const name = names.get(value);
  if (name === undefined) throw new Error(`unknown line type ${value}`);
  return name;
}

/** `centralConnection` of the DB: `0` is `NONE`; `undefined` is `null`. */
const CENTRAL_CONNECTIONS: Record<number, string> = {
  0: 'NONE',
  59: 'CENTRAL_CONNECTION',
  60: 'CENTRAL_CONNECTION_REVERSE',
  61: 'CENTRAL_CONNECTION_DUAL',
};

/** `PLACEMENT` of the DB: `left of` (0), `right of` (1), `over` (2). */
const PLACEMENTS: Record<number, string> = { 0: 'LEFT_OF', 1: 'RIGHT_OF', 2: 'OVER' };

function named(table: Record<number, string>, value: number | undefined): string | null {
  if (value === undefined) return null;
  const name = table[value];
  if (name === undefined) throw new Error(`unknown value ${value}`);
  return name;
}

function messageJson(message: ReferenceMessage, lineTypes: Map<number, string>) {
  const text = message.message;
  return {
    id: message.id,
    from: nullable(message.from),
    to: nullable(message.to),
    message:
      typeof text === 'string'
        ? text
        : {
            start: nullable(text.start),
            step: nullable(text.step),
            visible: text.visible,
          },
    wrap: message.wrap,
    type: lineTypeName(lineTypes, message.type),
    activate: nullable(message.activate),
    centralConnection: named(CENTRAL_CONNECTIONS, message.centralConnection),
    placement: named(PLACEMENTS, message.placement),
  };
}

await runExporter(loadModules, (input, modules) => {
  const { config, preprocess, detectType, encode, sequence } = modules;

  // The configuration of this input, as `mermaidAPI`'s `processAndSetConfigs` sets it.
  config.reset();
  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
  } catch {
    diagramType = undefined;
  }
  config.setDiagramConfigScope(diagramType);
  config.addDirective(processed.config ?? {});
  // Pinned last, so that no directive can change them: the reference's defaults.
  config.setConfig({ htmlLabels: true, securityLevel: 'strict' });

  // `Diagram.fromText`: the DB is cleared before the front matter title is set, as the clear
  // resets the title (`commonDb.ts`).
  const { db, parser, init } = sequence.diagram;
  db.clear();
  init(config.getConfig());
  if (processed.title) {
    db.setDiagramTitle(processed.title);
  }
  parser.parser.yy = db;
  parser.parse(encode.encodeEntities(processed.code.cleaned) + '\n');

  const boxes = db.getBoxes();
  const lineTypes = new Map(Object.entries(db.LINETYPE).map(([name, value]) => [value, name]));
  const boxIndex = (box: ReferenceBox | undefined): number | null => {
    if (box === undefined) return null;
    const index = boxes.indexOf(box);
    if (index < 0) throw new Error('an actor refers to a box the DB does not list');
    return index;
  };

  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    actors: [...db.getActors()].map(([id, actor]) => ({
      id,
      name: actor.name,
      description: actor.description,
      type: actor.type,
      wrap: actor.wrap,
      box: boxIndex(actor.box),
      prevActor: nullable(actor.prevActor),
      nextActor: nullable(actor.nextActor),
      // As `[key, value]` pairs in the object's key order, the order of the actor's menu: the
      // differential run compares objects without regard to their key order.
      links: Object.entries(actor.links),
      properties: actor.properties,
    })),
    boxes: boxes.map((box) => ({
      name: nullable(box.name),
      wrap: box.wrap,
      fill: box.fill,
      actorKeys: box.actorKeys,
    })),
    messages: db.getMessages().map((message) => messageJson(message, lineTypes)),
    createdActors: Object.fromEntries(db.getCreatedActors()),
    destroyedActors: Object.fromEntries(db.getDestroyedActors()),
  };
});
