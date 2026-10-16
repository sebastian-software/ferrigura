/**
 * Differential run for the configuration. The pie and flowchart inputs of `inputs.ts` and every
 * explicit case of `config-cases.ts` go through the TypeScript reference (`export-config.ts`) and
 * through ferrigura (`cargo run --example config`). Each side detects the diagram type from the
 * text, as a render does, so each input gets its own diagram scope: the reference through
 * `processAndSetConfigs`, ferrigura through `detect`. A flowchart input must come out in the
 * `flowchart` scope on both sides, and a pie input or case in `pie`.
 *
 * The key set that directives may use is compared first. Then, per input, the scope and the
 * effective configuration: the top level (with `theme`, `look` and `layout` resolved), `elk`, the
 * `pie` section and the section of the input's own scope. A rejection on both sides counts as
 * agreement.
 *
 * Usage: `pnpm port:diff config`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { configCases } from './config-cases.ts';
import { collectInputs } from './inputs.ts';
import { exceptions } from './exceptions.ts';

interface ConfigInput {
  id: number;
  source: string;
  text: string;
  site?: Record<string, unknown>;
  /** The collection the input came from: `pie`, `flowchart` or `case`. */
  group: string;
  /** The scope the input must be run in. */
  expectedScope: string;
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
const flowchartInputs = await collectInputs('flowchart');
const ganttInputs = await collectInputs('gantt');
const inputs: ConfigInput[] = [
  ...pieInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'pie',
    expectedScope: 'pie',
  })),
  ...flowchartInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'flowchart',
    expectedScope: 'flowchart',
  })),
  ...ganttInputs.map((input) => ({
    id: 0,
    source: input.source,
    text: input.text,
    group: 'gantt',
    expectedScope: 'gantt',
  })),
  ...configCases.map((testCase) => ({
    id: 0,
    source: testCase.source,
    text: testCase.text,
    site: testCase.site,
    group: 'case',
    expectedScope: 'pie',
  })),
].map((input, id) => ({ ...input, id }));

await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(inputs, null, 2));

run('pnpm', ['exec', 'tsx', 'tools/port/export-config.ts', inputPath, jsPath]);
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
const missingInRust = js.configKeys.filter((key) => !rsKeys.has(key));
const extraInRust = rs.configKeys.filter((key) => !jsKeys.has(key));
if (missingInRust.length > 0 || extraInRust.length > 0) {
  console.log(
    `\n✗ configKeys: ${missingInRust.length} missing in Rust, ${extraInRust.length} extra`
  );
  console.log(`  missing in Rust: ${JSON.stringify(missingInRust)}`);
  console.log(`  extra in Rust: ${JSON.stringify(extraInRust)}`);
}

/**
 * The diagram sections whose defaults the port does not own yet. The reference fills them with its
 * own generator (`.build/jsonSchema.ts`), which applies a section's own defaults only, and the
 * TypeScript defaults replace some sections whole; ferrigura takes them from the schema. A section
 * is compared only for an input of its own diagram type: `pie` and `flowchart` are ported, and
 * `info` has no section. Every other section is left out.
 */
const UNPORTED_SECTIONS = [
  'flowchart',
  'swimlane',
  'agentflow',
  'sequence',
  'journey',
  'class',
  'state',
  'er',
  'quadrantChart',
  'xyChart',
  'requirement',
  'mindmap',
  'ishikawa',
  'kanban',
  'timeline',
  'gitGraph',
  'c4',
  'sankey',
  'block',
  'packet',
  'treeView',
  'architecture',
  'eventmodeling',
  'radar',
  'usecase',
  'venn',
  'wardley-beta',
  'cynefin',
  'railroad',
  'treemap',
];

/**
 * The output with the sections that are not compared removed: the sections of diagram types that
 * are not ported, and the sections of the other ported types. `scope` is the section of the input.
 */
function ported(config: unknown, scope: string | null): unknown {
  if (typeof config !== 'object' || config === null) return config;
  const copy = { ...(config as Record<string, unknown>) };
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
// The named exceptions of the gantt collection (exceptions.ts) apply to the same inputs here.
const namedExceptions = new Set((exceptions.gantt ?? []).map((entry) => entry.text));
let namedExcepted = 0;
const groupCounts = new Map<string, number>();
for (const input of inputs) {
  groupCounts.set(input.group, (groupCounts.get(input.group) ?? 0) + 1);
}
const acceptedByGroup = new Map<string, number>();
for (const input of inputs) {
  if (namedExceptions.has(input.text)) {
    namedExcepted += 1;
    continue;
  }
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
  if (agree && scopeIsExpected) continue;
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
    `${accepted} accepted, ${differences} differences, ${namedExcepted} named exceptions; ` +
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

if (differences > 0 || missingInRust.length > 0 || extraInRust.length > 0) {
  process.exit(1);
}
