// cspell:ignore writeback
/**
 * The swimlane reference corpus: the format of its cases, the snapshot of the layout data they hold,
 * and the writing of the cases and the manifest. Shared by `capture-corpus.ts` (Chromium) and
 * `capture-ddlt.spec.ts` (DDLT); the commands are in the header of `capture-corpus.ts`.
 *
 * A stage report carries JSON copies of the layout's `direction`, `config`, `nodes` and `edges` (see
 * `packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/stageReport.ts`). This module keeps,
 * per node and per edge, the keys in the key set (every identifier of the swimlane sources) and the
 * keys whose value differs from the input, and it keeps the configuration keys the folder reads.
 */
import { mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
export const SWIMLANE_SOURCE_DIR = join(
  REPO_ROOT,
  'packages/mermaid/src/rendering-util/layout-algorithms/swimlanes'
);
export const CORPUS_DIR = join(REPO_ROOT, 'crates/ferrigura-swimlane/tests/corpus');
/** Where the DDLT part and the browser part meet (gitignored, under target/). */
export const PARTS_DIR = join(REPO_ROOT, 'target/swimlane-corpus-parts');

/** Version of the case and manifest format. Bump it when either changes shape. */
export const CORPUS_VERSION = 1;
const JSON_INDENT = 1;

/**
 * The configuration keys the swimlane folder reads (non-spec files): `flowchart` and `swimlane` in
 * `layoutCore.ts:22-26` (and `swimlane.lineHops` in `adjustLayout.ts:9`), and `look` in `helpers.ts:125`.
 * `capture-corpus.ts` checks this list against a grep of the folder before it writes anything.
 */
export const CONFIG_KEYS = ['flowchart', 'look', 'swimlane'] as const;

export const NON_DEFAULT_REASON =
  'non-default option (ignoreCrossLaneEdges false / automaticLaneOrdering true): not ported';

export type CaseGroup = 'fixture' | 'ddlt' | 'generated';
export type Direction = 'TB' | 'LR' | 'BT' | 'RL';
export type Json = null | boolean | number | string | Json[] | JsonObject;
export interface JsonObject {
  [key: string]: Json;
}

/** One stage report as the page or the DDLT run received it. */
export interface StageRecord {
  stage: string;
  data: unknown;
}

export interface Snapshot {
  direction: string | null;
  config: JsonObject;
  nodes: JsonObject[];
  edges: JsonObject[];
}

export interface CorpusStage {
  stage: string;
  data: unknown;
}

export interface CorpusCase {
  name: string;
  group: CaseGroup;
  source: string;
  text: string;
  input: Snapshot;
  stages: CorpusStage[];
  output: Snapshot;
  returnedDirection: Direction;
}

export interface Exclusion {
  source: string;
  reason: string;
}

export interface CaptureResult {
  cases: CorpusCase[];
  excluded: Exclusion[];
}

const DIRECTIONS: readonly string[] = ['TB', 'LR', 'BT', 'RL'];
const NON_FINITE_MARK = '__nonFinite__:';

function byText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** The id of a node or an edge as a string (ids are strings or numbers in the layout data). */
function idOf(value: Json | undefined): string {
  return typeof value === 'string' || typeof value === 'number'
    ? String(value)
    : JSON.stringify(value);
}

/**
 * The key set: every identifier that appears as a word token in a non-spec `.ts` file of the swimlane
 * folder (`direction/` and `orthogonalRouter/` included, comments and strings included), sorted by code point.
 */
export function identifierKeySet(sourceDir: string = SWIMLANE_SOURCE_DIR): string[] {
  const words = new Set<string>();
  for (const file of sourceFiles(sourceDir)) {
    for (const match of readFileSync(file, 'utf8').matchAll(/[$A-Z_a-z][\w$]*/g)) {
      words.add(match[0]);
    }
  }
  return [...words].sort(byText);
}

/** The non-spec `.ts` files under a folder, `__tests__` left out, sorted by name at each level. */
export function sourceFiles(dir: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) =>
    byText(a.name, b.name)
  )) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== '__tests__') {
        files.push(...sourceFiles(full));
      }
    } else if (entry.isFile() && entry.name.endsWith('.ts') && !entry.name.endsWith('.spec.ts')) {
      files.push(full);
    }
  }
  return files;
}

/** A node or an edge: its keys in the key set, and the keys whose value differs from the input's object. */
function keptKeys(
  object: JsonObject,
  base: JsonObject | undefined,
  keys: ReadonlySet<string>
): JsonObject {
  const kept: JsonObject = {};
  for (const [key, value] of Object.entries(object)) {
    const changed =
      base === undefined || !(key in base) || JSON.stringify(base[key]) !== JSON.stringify(value);
    if (keys.has(key) || changed) {
      kept[key] = value;
    }
  }
  return kept;
}

function asLayout(data: unknown): Snapshot {
  return data as Snapshot;
}

/**
 * The snapshot of a stage. A node is compared with the node at the same index of the input, and an
 * edge with the input edge of the same id: the router drops the layout-only edges from the edge list
 * (`orthogonalRouter/router.ts`, the last statement of `routeEdgesOrthogonal`), so the edge list is an
 * ordered subset of the input's from `routed` on.
 */
function snapshotOf(raw: Snapshot, input: Snapshot, keys: ReadonlySet<string>): Snapshot {
  const inputEdges = new Map(input.edges.map((edge) => [idOf(edge.id), edge]));
  return {
    direction: raw.direction ?? null,
    config: Object.fromEntries(
      CONFIG_KEYS.filter((key) => key in raw.config).map((key) => [key, raw.config[key]])
    ),
    nodes: raw.nodes.map((node, i) => keptKeys(node, input.nodes[i], keys)),
    edges: raw.edges.map((edge) => keptKeys(edge, inputEdges.get(idOf(edge.id)), keys)),
  };
}

/**
 * The nodes keep their order and ids. The edges are an ordered subset of the input's, by id, and
 * their ids are unique. Anything else cannot be compared with the input, so the call is refused.
 */
function shapeProblem(raw: Snapshot, input: Snapshot): string | undefined {
  const before = input.nodes;
  if (before.length !== raw.nodes.length) {
    return `nodes: ${before.length} at the input, ${raw.nodes.length} later`;
  }
  const movedNode = raw.nodes.findIndex((node, i) => node.id !== before[i].id);
  if (movedNode >= 0) {
    return `nodes: the item at index ${movedNode} changed its id`;
  }
  const inputIds = input.edges.map((edge) => idOf(edge.id));
  const position = new Map(inputIds.map((id, i) => [id, i]));
  let last = -1;
  for (const edge of raw.edges) {
    const at = position.get(idOf(edge.id));
    if (at === undefined) {
      return `edges: edge ${idOf(edge.id)} is not in the input`;
    }
    if (at <= last) {
      return `edges: the order of ${idOf(edge.id)} differs from the input`;
    }
    last = at;
  }
  return undefined;
}

/**
 * One case from the stage reports of one core call. The reason is returned instead when the call is
 * not a corpus case (a non-default option, a call that did not finish, a non-finite number).
 */
export function buildCase(
  name: string,
  group: CaseGroup,
  source: string,
  text: string,
  records: readonly StageRecord[],
  keys: ReadonlySet<string>
): { case: CorpusCase } | { reason: string } {
  if (records[0]?.stage !== 'input') {
    return { reason: 'the layout core was not entered (no input stage)' };
  }
  const last = records[records.length - 1];
  if (records.length < 2 || last.stage !== 'output') {
    return { reason: `the layout core did not finish (last stage: ${last.stage})` };
  }
  const input = asLayout(records[0].data);
  if (new Set(input.edges.map((edge) => idOf(edge.id))).size !== input.edges.length) {
    return { reason: 'duplicate edge ids in the input' };
  }
  const swimlaneConfig = input.config.swimlane as JsonObject | undefined;
  if (
    swimlaneConfig?.ignoreCrossLaneEdges === false ||
    swimlaneConfig?.automaticLaneOrdering === true
  ) {
    return { reason: NON_DEFAULT_REASON };
  }
  const middle = records.slice(1, -1);
  const names = middle.map((record) => record.stage);
  const orderOk =
    names.slice(0, 3).join(',') === 'sugiyama,writeback,routed' &&
    names.slice(3).every((stage) => stage.startsWith('post:'));
  if (!orderOk) {
    return { reason: `unexpected stage order: ${names.join(', ')}` };
  }
  const output = asLayout(last.data);
  for (const record of [records[0], ...middle.filter((r) => r.stage !== 'sugiyama'), last]) {
    const problem = shapeProblem(asLayout(record.data), input);
    if (problem !== undefined) {
      return { reason: `stage ${record.stage} changed the item lists: ${problem}` };
    }
  }
  // The stages sugiyama, writeback and routed are always written. A post-processing pass is written
  // only when its snapshot differs from the one before it: most passes change nothing in most
  // diagrams, and writing every pass made the corpus too large (about 70 MB). The names keep the
  // numbering the hook gave them, so a missing name is a pass that changed nothing.
  const stages: CorpusStage[] = [];
  let previous: string | undefined;
  for (const record of middle) {
    if (record.stage === 'sugiyama') {
      stages.push({ stage: record.stage, data: record.data });
      continue;
    }
    const snapshot = snapshotOf(asLayout(record.data), input, keys);
    const text = JSON.stringify(snapshot);
    if (!record.stage.startsWith('post:') || text !== previous) {
      stages.push({ stage: record.stage, data: snapshot });
    }
    previous = text;
  }
  const inputSnapshot = snapshotOf(input, input, keys);
  const outputSnapshot = snapshotOf(output, input, keys);
  for (const item of [
    { stage: 'input', data: inputSnapshot },
    ...stages,
    { stage: 'output', data: outputSnapshot },
  ]) {
    if (JSON.stringify(item.data).includes(`"${NON_FINITE_MARK}`)) {
      return { reason: `a non-finite number in stage ${item.stage}` };
    }
  }
  const direction = outputSnapshot.direction ?? 'TB';
  if (!DIRECTIONS.includes(direction)) {
    return { reason: `the layout direction ${direction} is not a swimlane direction` };
  }
  return {
    case: {
      name,
      group,
      source,
      text,
      input: inputSnapshot,
      stages,
      output: outputSnapshot,
      returnedDirection: direction as Direction,
    },
  };
}

/** The records of a page or a run, split into one list per core call, at each input stage. */
export function splitCalls(records: readonly StageRecord[]): StageRecord[][] {
  const calls: StageRecord[][] = [];
  for (const record of records) {
    if (record.stage === 'input' || calls.length === 0) {
      calls.push([]);
    }
    calls[calls.length - 1].push(record);
  }
  return calls;
}

/**
 * The cases of one source file: one case per core call, named `<name>` when there is one call and
 * `<name>__<k>` (k from 0) when there are several.
 */
export function casesFromCalls(args: {
  baseName: string;
  group: CaseGroup;
  source: string;
  text: string;
  calls: readonly (readonly StageRecord[])[];
  keys: ReadonlySet<string>;
}): CaptureResult {
  const result: CaptureResult = { cases: [], excluded: [] };
  if (args.calls.length === 0) {
    result.excluded.push({ source: args.source, reason: 'the layout core was not called' });
    return result;
  }
  args.calls.forEach((records, k) => {
    const name = args.calls.length === 1 ? args.baseName : `${args.baseName}__${k}`;
    const built = buildCase(name, args.group, args.source, args.text, records, args.keys);
    if ('reason' in built) {
      result.excluded.push({ source: args.source, reason: built.reason });
    } else {
      result.cases.push(built.case);
    }
  });
  return result;
}

function caseFileContent(record: CorpusCase): string {
  const body = {
    name: record.name,
    group: record.group,
    source: record.source,
    text: record.text,
    input: record.input,
    stages: record.stages,
    output: record.output,
    returnedDirection: record.returnedDirection,
  };
  return `${JSON.stringify(body, null, JSON_INDENT)}\n`;
}

/**
 * Clears the directory of JSON and writes the cases and the manifest. Returns the size of the JSON
 * files in bytes. Only `.json` files are touched, so a re-run leaves no stale case.
 */
export function writeCorpus(
  dir: string,
  cases: readonly CorpusCase[],
  excluded: readonly Exclusion[],
  keySet: readonly string[]
): number {
  const sorted = [...cases].sort((a, b) => byText(a.name, b.name));
  if (new Set(sorted.map((record) => record.name)).size !== sorted.length) {
    throw new Error('Duplicate case names: two captures share a name');
  }
  mkdirSync(dir, { recursive: true });
  for (const file of readdirSync(dir)) {
    if (file.endsWith('.json')) {
      rmSync(join(dir, file));
    }
  }
  for (const record of sorted) {
    writeFileSync(join(dir, `${record.name}.json`), caseFileContent(record));
  }
  const manifest = {
    version: CORPUS_VERSION,
    keySet,
    configKeys: [...CONFIG_KEYS],
    cases: sorted.map((record) => ({
      name: record.name,
      group: record.group,
      source: record.source,
      nodes: record.input.nodes.length,
      edges: record.input.edges.length,
      stages: record.stages.length,
    })),
    excluded: [...excluded].sort(
      (a, b) => byText(a.source, b.source) || byText(a.reason, b.reason)
    ),
  };
  writeFileSync(join(dir, 'manifest.json'), `${JSON.stringify(manifest, null, JSON_INDENT)}\n`);
  return readdirSync(dir)
    .filter((file) => file.endsWith('.json'))
    .reduce((sum, file) => sum + statSync(join(dir, file)).size, 0);
}
