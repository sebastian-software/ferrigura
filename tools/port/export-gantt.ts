/**
 * Canonical JSON of gantt diagrams, from the TypeScript reference.
 *
 * The text goes through the steps `Diagram.fromText` runs for every diagram: `preprocessDiagram`,
 * `db.clear()`, the front matter title through `setDiagramTitle`, then `encodeEntities` and a line
 * break for the Jison parser. The parser fills the DB, and `getTasks()` gives the tasks. The model
 * is the DB's state as `docs-port/gantt/PLAN.md`, section 6, defines it.
 *
 * Each input has a DB of its own. `ganttDb.js` is loaded under a query no other input uses, so
 * Vite evaluates a new module instance with its own `taskDb` (the reference's `clear()` keeps
 * `taskDb`, so one input would see the task ids of the inputs before it). The Jison parser's `yy`
 * is that instance, as `Diagram.fromText` sets it.
 *
 * The run is in UTC, and the clock is fixed: `new Date()`, `Date.now()` and dayjs (which reads
 * `new Date()` for the parts a format leaves out) all give `NOW`.
 *
 * Usage: `tsx tools/port/export-gantt.ts inputs.json outputs.json` (the run sets `TZ=UTC` itself).
 */
process.env.TZ = 'UTC';

import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

/** The instant of the clock: 1010-10-10T00:00Z, the date of the gantt e2e clock. */
const NOW = Date.UTC(1010, 9, 10);

/**
 * A `Date` whose no-argument constructor and `now()` give `NOW`. The other forms pass through, and
 * its instances are `Date`s, so `instanceof Date` holds on both sides.
 */
class FixedDate extends Date {
  constructor(...args: unknown[]) {
    super(...(args.length === 0 ? [NOW] : (args as [number])));
  }

  static override now(): number {
    return NOW;
  }
}

interface GanttTask {
  id: string;
  task: string;
  section: string;
  order: number;
  vert?: boolean;
  milestone?: boolean;
  active?: boolean;
  done?: boolean;
  crit?: boolean;
  classes: string[];
  manualEndTime?: boolean;
  startTime?: Date;
  endTime?: Date;
  renderEndTime?: Date | null;
}

/** The DB's default export, as `Diagram.fromText` uses it for `yy` and the getters. */
interface GanttDb {
  clear(): void;
  setDiagramTitle(text: string): void;
  setWeekend(day: string): void;
  getDiagramTitle(): string;
  getAccTitle(): string;
  getAccDescription(): string;
  getDateFormat(): string;
  getAxisFormat(): string;
  getTickInterval(): string | undefined;
  getTodayMarker(): string;
  topAxisEnabled(): boolean;
  endDatesAreInclusive(): boolean;
  getWeekday(): string;
  getIncludes(): string[];
  getExcludes(): string[];
  getSections(): string[];
  getLinks(): Map<string, string>;
  getTasks(): GanttTask[];
}

interface Modules {
  reference: Reference;
  preprocess: {
    preprocessDiagram(code: string): { code: { cleaned: string }; title?: string };
  };
  encode: { encodeEntities(text: string): string };
  parser: { default: { parser: { yy: unknown }; parse(text: string): unknown } };
}

async function loadModules(reference: Reference): Promise<Modules> {
  globalThis.Date = FixedDate as unknown as DateConstructor;
  provideDomForSanitising();
  await registerDetectors(reference);
  const [preprocess, encode, parser] = await Promise.all([
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['parser']>('/packages/mermaid/src/diagrams/gantt/parser/gantt.jison'),
  ]);
  return { reference, preprocess, encode, parser };
}

/** The epoch milliseconds of a date, or `null` for an unresolved value or an Invalid Date. */
const timeOf = (value: Date | null | undefined): number | null => {
  if (!(value instanceof Date)) return null;
  const time = value.getTime();
  return Number.isNaN(time) ? null : time;
};

let instances = 0;

await runExporter(loadModules, async (input, { reference, preprocess, encode, parser }) => {
  instances += 1;
  const ganttDb = await reference.load<{ default: GanttDb }>(
    `/packages/mermaid/src/diagrams/gantt/ganttDb.js?run=${instances}`
  );
  const db = ganttDb.default;

  // The DB keeps the weekend in a variable that has no getter, so the statement's value is read as
  // it is given to `setWeekend`. Everything else is the DB itself.
  let weekend = 'saturday';
  const yy = {
    ...db,
    setWeekend(day: string) {
      weekend = day;
      db.setWeekend(day);
    },
  };

  const { code, title } = preprocess.preprocessDiagram(input.text);
  db.clear();
  if (title) {
    db.setDiagramTitle(title);
  }
  parser.default.parser.yy = yy;
  parser.default.parse(encode.encodeEntities(code.cleaned) + '\n');

  return {
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    dateFormat: db.getDateFormat(),
    axisFormat: db.getAxisFormat(),
    tickInterval: db.getTickInterval() ?? null,
    todayMarker: db.getTodayMarker(),
    topAxis: db.topAxisEnabled(),
    inclusiveEndDates: db.endDatesAreInclusive(),
    weekday: db.getWeekday(),
    weekend,
    includes: db.getIncludes(),
    excludes: db.getExcludes(),
    sections: db.getSections(),
    links: Object.fromEntries(db.getLinks()),
    tasks: db.getTasks().map((task) => ({
      id: task.id,
      task: task.task,
      section: task.section,
      order: task.order,
      vert: task.vert === true,
      milestone: task.milestone === true,
      active: task.active === true,
      done: task.done === true,
      crit: task.crit === true,
      classes: task.classes,
      manualEndTime: task.manualEndTime === true,
      startTime: timeOf(task.startTime),
      endTime: timeOf(task.endTime),
      renderEndTime: timeOf(task.renderEndTime),
    })),
  };
});
