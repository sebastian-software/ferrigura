/**
 * Differential run for the configuration: every pie input and every explicit case of
 * `config-cases.ts` goes through the TypeScript reference (`export-config.ts`) and through
 * ferrigura (`cargo run --example config`). The key set that directives may use is compared first,
 * then the effective configuration of each input. A rejection on both sides counts as agreement.
 *
 * Usage: `pnpm port:diff config`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { configCases } from './config-cases.ts';
import { collectInputs } from './inputs.ts';

interface ConfigInput {
  id: number;
  source: string;
  text: string;
  site?: Record<string, unknown>;
}

interface ConfigOutput {
  id: number;
  ok: boolean;
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
const inputs: ConfigInput[] = [
  ...pieInputs.map((input) => ({ id: 0, source: input.source, text: input.text })),
  ...configCases.map((testCase) => ({
    id: 0,
    source: testCase.source,
    text: testCase.text,
    site: testCase.site,
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
 * TypeScript defaults replace some sections whole; ferrigura takes them from the schema. They are
 * compared when their diagram type is ported (`pie` is, and `info` has no section).
 */
const UNPORTED_SECTIONS = [
  'flowchart',
  'swimlane',
  'agentflow',
  'sequence',
  'gantt',
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

/** The output with the sections of diagram types that are not ported removed. */
function ported(config: unknown): unknown {
  if (typeof config !== 'object' || config === null) return config;
  const copy = { ...(config as Record<string, unknown>) };
  for (const section of UNPORTED_SECTIONS) delete copy[section];
  return copy;
}

const jsById = new Map(js.results.map((output) => [output.id, output]));
const rsById = new Map(rs.results.map((output) => [output.id, output]));

let accepted = 0;
let differences = 0;
for (const input of inputs) {
  const jsOutput = jsById.get(input.id);
  const rsOutput = rsById.get(input.id);
  if (jsOutput?.ok) accepted += 1;
  const agree =
    jsOutput !== undefined &&
    rsOutput !== undefined &&
    jsOutput.ok === rsOutput.ok &&
    (!jsOutput.ok || same(ported(jsOutput.config), ported(rsOutput.config)));
  if (agree) continue;
  differences += 1;
  console.log(`\n✗ ${input.source}`);
  console.log(`  input: ${JSON.stringify(input.text.slice(0, 300))}`);
  if (input.site) console.log(`  site: ${JSON.stringify(input.site)}`);
  console.log(`  js: ${JSON.stringify(jsOutput).slice(0, 1200)}`);
  console.log(`  rs: ${JSON.stringify(rsOutput).slice(0, 1200)}`);
}

console.log(
  `\nconfig: ${inputs.length} inputs, ${accepted} accepted by the reference, ` +
    `${differences} differences; configKeys: ${missingInRust.length + extraInRust.length} differences`
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
