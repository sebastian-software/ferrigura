/**
 * Differential run for the configuration. The inputs of `inputs.ts` of the diagram types that
 * ferrigura draws, and every explicit case of `config-cases.ts`, go through the TypeScript reference
 * (`export-config.ts`) and through ferrigura (`cargo run --example config`). Each side detects the
 * diagram type from the text, as a render does, so each input gets its own diagram scope: the
 * reference through `processAndSetConfigs`, ferrigura through `detect`. A flowchart input must come
 * out in the `flowchart` scope on both sides, a state input in `state`, a pie input or case in `pie`,
 * and a mindmap, XY chart or quadrant chart input in its own scope, whose section is compared too.
 *
 * The key set that directives may use is compared first. Then, per input, the scope and the
 * effective configuration: the top level (with `theme`, `look` and `layout` resolved), `elk`, the
 * `pie` section and the section of the input's own scope. A rejection on both sides counts as
 * agreement.
 *
 * Usage: `pnpm -C tools/port port:diff config`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { configCases } from './config-cases.ts';
import { undetectedCases } from './cases.ts';
import { exceptions } from './exceptions.ts';
import { INERT_KEYS, withoutInertOptions } from './inert-options.ts';
import { collectInputs, looksLikeType } from './inputs.ts';

interface ConfigInput {
  id: number;
  source: string;
  text: string;
  site?: Record<string, unknown>;
  /** The collection the input came from: the diagram type as `inputs.ts` collects it, or `case`. */
  group: string;
  /** The scope the input must be run in, or `null` for an input that no detector takes. */
  expectedScope: string | null;
}

interface ConfigOutput {
  id: number;
  ok: boolean;
  scope?: string | null;
  config?: unknown;
  error?: string;
}

interface ConfigRun {
  configKeys: string[];
  results: ConfigOutput[];
}

const root = fileURLToPath(new URL('../../', import.meta.url));
const workDir = `${root}target/port`;
const inputPath = `${workDir}/config-inputs.json`;
const jsPath = `${workDir}/config-js.json`;
const rsPath = `${workDir}/config-rs.json`;

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.status !== 0) {
    console.error(`${command} ${args.join(' ')} failed`);
    process.exit(2);
  }
}

/** Equality of two JSON values; numbers compare with `===`, so -0 and 0 are equal. */
function same(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const keysA = Object.keys(a);
  const keysB = Object.keys(b);
  if (keysA.length !== keysB.length) return false;
  return keysA.every(
    (key) =>
      Object.hasOwn(b, key) &&
      same((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key])
  );
}

const pieInputs = await collectInputs('pie');
const undetectedPie = new Set(undetectedCases.pie ?? []);
const flowchartInputs = await collectInputs('flowchart');
const erInputs = await collectInputs('er');
const classInputs = await collectInputs('class');
const sequenceInputs = await collectInputs('sequence');
// The state group is the inputs the state detector accepts: a case that is not a state diagram
// belongs to no group, and the reference's config of it is in no diagram scope.
const stateInputs = (await collectInputs('state')).filter((input) =>
  looksLikeType('state', input.text)
);
const ganttInputs = await collectInputs('gantt');
/**
 * The types whose own section is compared, each with its group name (as `inputs.ts` collects it) and
 * its configuration section (`getDiagramConfigKey`). Their inputs must be detected in that scope on
 * both sides.
 */
const scopedGroups: [group: string, scope: string][] = [
  ['mindmap', 'mindmap'],
  ['xychart', 'xyChart'],
  ['quadrantChart', 'quadrantChart'],
];
const scopedInputs: ConfigInput[] = [];
for (const [group, scope] of scopedGroups) {
  for (const input of await collectInputs(group)) {
    scopedInputs.push({
      id: 0,
      source: input.source,
      text: input.text,
      group,
      expectedScope: (undetectedCases[group] ?? []).includes(input.text) ? null : scope,
    });
  }
}
const inputs: ConfigInput[] = [
  ...pieInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'pie',
    expectedScope: undetectedPie.has(input.text) ? null : 'pie',
  })),
  ...flowchartInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'flowchart',
    expectedScope: 'flowchart',
  })),
  ...erInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'er',
    expectedScope: 'er',
  })),
  ...classInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'class',
    expectedScope: 'class',
  })),
  ...sequenceInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'sequence',
    expectedScope: 'sequence',
  })),
  ...stateInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'state',
    expectedScope: 'state',
  })),
  ...ganttInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'gantt',
    expectedScope: 'gantt',
  })),
  ...scopedInputs,
  ...configCases.map((testCase) => ({
    id: 0,
    source: testCase.source,
    text: testCase.text,
    site: testCase.site,
    group: 'case',
    expectedScope: testCase.scope ?? 'pie',
  })),
].map((input, id) => ({ ...input, id }));

await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(inputs, null, 2));

run('pnpm', ['-C', 'tools/port', 'exec', 'tsx', 'export-config.ts', inputPath, jsPath]);
run('cargo', [
  'run',
  '--quiet',
  '-p',
  'ferrigura-syntax',
  '--example',
  'config',
  '--',
  inputPath,
  rsPath,
]);

const js = JSON.parse(await readFile(jsPath, 'utf8')) as ConfigRun;
const rs = JSON.parse(await readFile(rsPath, 'utf8')) as ConfigRun;

const jsKeys = new Set(js.configKeys);
const rsKeys = new Set(rs.configKeys);
const missingInRust = js.configKeys.filter((key) => !rsKeys.has(key) && !INERT_KEYS.has(key));
const extraInRust = rs.configKeys.filter((key) => !jsKeys.has(key) && !INERT_KEYS.has(key));
if (missingInRust.length > 0 || extraInRust.length > 0) {
  console.log(
    `\n✗ configKeys: ${missingInRust.length} missing in Rust, ${extraInRust.length} extra`
  );
  console.log(`  missing in Rust: ${JSON.stringify(missingInRust)}`);
  console.log(`  extra in Rust: ${JSON.stringify(extraInRust)}`);
}

/**
 * The diagram sections that are compared only for an input of their own diagram type. The reference
 * fills them with its own generator (`.build/jsonSchema.ts`), which applies a section's own defaults
 * only, and the TypeScript defaults replace some sections whole; ferrigura takes them from the
 * schema. The sections of the types that ferrigura does not draw have no input of their type, so
 * they are never compared.
 */
const UNPORTED_SECTIONS = [
  'flowchart',
  'swimlane',
  'agentflow',
  'class',
  'state',
  'quadrantChart',
  'xyChart',
  'ishikawa',
  'timeline',
  'gitGraph',
  'c4',
  'sankey',
  'packet',
  'treeView',
  'radar',
  'usecase',
  'venn',
  'wardley-beta',
  'railroad',
  'treemap',
];

/**
 * The output with the sections that are not compared removed: every section of `UNPORTED_SECTIONS`
 * other than `scope`, the section of the input.
 */
function ported(config: unknown, scope: string | null): unknown {
  if (typeof config !== 'object' || config === null) return config;
  const copy = { ...(withoutInertOptions(config) as Record<string, unknown>) };
  for (const section of UNPORTED_SECTIONS) {
    if (section !== scope) delete copy[section];
  }
  return copy;
}

const jsById = new Map(js.results.map((output) => [output.id, output]));
const rsById = new Map(rs.results.map((output) => [output.id, output]));

/**
 * The themes ferrigura ships. Its theme variables are compared only for these: any other theme name
 * is drawn with one of the two by darkness (`ferrigura_model::theme::Theme::from_name`), so the
 * variables of the reference's other themes are not what ferrigura writes.
 */
const SHIPPED_THEMES = ['redux-color', 'redux-dark-color'];

let accepted = 0;
let differences = 0;
let unshippedThemes = 0;
/** The named exceptions of the run (`exceptions.ts`, key `config`), by input text. */
const named = new Map((exceptions.config ?? []).map((entry) => [entry.text, entry.reason]));
let excepted = 0;
const stale: string[] = [];
const groupCounts = new Map<string, number>();
for (const input of inputs) {
  groupCounts.set(input.group, (groupCounts.get(input.group) ?? 0) + 1);
}
const acceptedByGroup = new Map<string, number>();
for (const input of inputs) {
  const jsOutput = jsById.get(input.id);
  const rsOutput = rsById.get(input.id);
  if (jsOutput?.ok) {
    accepted += 1;
    acceptedByGroup.set(input.group, (acceptedByGroup.get(input.group) ?? 0) + 1);
  }
  const scope = jsOutput?.ok ? (jsOutput.scope ?? null) : null;
  const jsConfig = jsOutput?.ok
    ? (ported(jsOutput.config, scope) as Record<string, unknown>)
    : undefined;
  const rsConfig = rsOutput?.ok
    ? (ported(rsOutput.config, scope) as Record<string, unknown>)
    : undefined;
  if (jsConfig && rsConfig && !SHIPPED_THEMES.includes(String(jsConfig.theme))) {
    unshippedThemes += 1;
    delete jsConfig.themeVariables;
    delete rsConfig.themeVariables;
  }
  const scopeAgrees = (jsOutput?.scope ?? null) === (rsOutput?.scope ?? null);
  const scopeIsExpected = !jsOutput?.ok || jsOutput.scope === input.expectedScope;
  const agree =
    jsOutput !== undefined &&
    rsOutput !== undefined &&
    jsOutput.ok === rsOutput.ok &&
    (!jsOutput.ok || (scopeAgrees && same(jsConfig, rsConfig)));
  if (agree && scopeIsExpected) {
    if (named.has(input.text)) stale.push(input.text);
    continue;
  }
  if (named.has(input.text)) {
    excepted += 1;
    continue;
  }
  differences += 1;
  console.log(`\n✗ ${input.source}`);
  console.log(`  input: ${JSON.stringify(input.text.slice(0, 300))}`);
  if (input.site) console.log(`  site: ${JSON.stringify(input.site)}`);
  if (!scopeIsExpected) {
    console.log(
      `  scope: the reference gives ${JSON.stringify(jsOutput?.scope)}, ` +
        `the input is collected as ${input.expectedScope}`
    );
  }
  console.log(`  js: ${JSON.stringify(jsOutput).slice(0, 1200)}`);
  console.log(`  rs: ${JSON.stringify(rsOutput).slice(0, 1200)}`);
}

const groups = [...groupCounts]
  .map(([group, count]) => `${group} ${acceptedByGroup.get(group) ?? 0}/${count}`)
  .join(', ');
console.log(
  `\nconfig: ${inputs.length} inputs (accepted by the reference per group: ${groups}), ` +
    `${accepted} accepted, ${excepted} named exceptions, ${differences} differences ` +
    `(${stale.length} stale exceptions); ` +
    `configKeys: ${missingInRust.length + extraInRust.length} differences; ` +
    `${unshippedThemes} inputs with a theme ferrigura does not ship compared without theme variables`
);

await writeFile(
  `${workDir}/config-summary.json`,
  JSON.stringify(
    {
      type: 'config',
      inputs: inputs.length,
      accepted,
      differences,
      configKeyDifferences: missingInRust.length + extraInRust.length,
    },
    null,
    2
  )
);

for (const text of stale) {
  console.log(`\n✗ stale exception, now agrees: ${JSON.stringify(text)}`);
}

if (differences > 0 || stale.length > 0 || missingInRust.length > 0 || extraInRust.length > 0) {
  process.exit(1);
}
