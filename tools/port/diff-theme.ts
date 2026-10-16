/**
 * Differential run for the theme variables: each shipped theme with each set of overrides goes
 * through the TypeScript reference (`export-theme.ts`) and through ferrigura
 * (`cargo run --example theme`). Both sides give the variables of the theme, and they must agree
 * key by key: the key sets are equal, strings are equal byte for byte and numbers with `===`. A
 * rejection on both sides (a colour that khroma cannot read) counts as agreement.
 *
 * Usage: `pnpm exec tsx tools/port/diff-theme.ts`
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const THEMES = ['redux-color', 'redux-dark-color'] as const;

/** Sets of overrides, named for the case they cover. `null` means no overrides. */
const OVERRIDE_CASES: [string, Record<string, unknown> | null][] = [
  ['no overrides', null],
  ['an empty object', {}],
  ['primaryColor', { primaryColor: '#ff0000' }],
  ['secondaryColor', { secondaryColor: '#00ff00' }],
  ['tertiaryColor', { tertiaryColor: '#0000ff' }],
  ['background', { background: '#222222' }],
  ['background black', { background: '#000000' }],
  ['background white', { background: '#ffffff' }],
  ['textColor', { textColor: '#abcdef' }],
  ['primaryTextColor', { primaryTextColor: '#123456' }],
  ['lineColor', { lineColor: '#ff00ff' }],
  ['cScale0', { cScale0: '#ff8800' }],
  ['pie1', { pie1: '#00ff88' }],
  ['fontFamily', { fontFamily: 'monospace' }],
  ['xyChart', { xyChart: { backgroundColor: '#eeeeee', titleColor: 'hsl(120, 50%, 50%)' } }],
  ['xyChart without keys', { xyChart: {} }],
  ['THEME_COLOR_LIMIT', { THEME_COLOR_LIMIT: 8 }],
  ['THEME_COLOR_LIMIT as text', { THEME_COLOR_LIMIT: '8' }],
  ['THEME_COLOR_LIMIT 0', { THEME_COLOR_LIMIT: 0 }],
  ['THEME_COLOR_LIMIT 13 without its colours', { THEME_COLOR_LIMIT: 13 }],
  [
    'THEME_COLOR_LIMIT 13 with its colours',
    {
      THEME_COLOR_LIMIT: 13,
      cScale12: '#333333',
      cScaleInv12: '#000000',
      cScalePeer12: '#111111',
      cScaleLabel12: '#222222',
    },
  ],
  [
    'rgb, hsl, keyword and rgba colours',
    {
      primaryColor: 'rgb(255, 0, 0)',
      secondaryColor: 'hsl(120, 50%, 50%)',
      tertiaryColor: 'white',
      lineColor: 'rgba(10, 20, 30, 0.5)',
    },
  ],
  ['three-digit and eight-digit hex', { primaryColor: '#abc', secondaryColor: '#11223344' }],
  ['an empty string', { textColor: '' }],
  ['empty strings of derived values', { primaryTextColor: '', secondaryColor: '' }],
  ['darkMode', { darkMode: true }],
  ['darkMode with a colour', { darkMode: true, primaryColor: '#334455' }],
  ['a keyword that is transparent', { altSectionBkgColor: 'transparent' }],
  ['a colour khroma cannot read', { primaryColor: 'unreadable' }],
  ['an array override', { borderColorArray: ['#010203', '#040506', '#070809'] }],
  ['a null override', { noteBkgColor: null }],
  ['numeric overrides', { pieOpacity: 0.5, strokeWidth: 3, fontWeight: 700 }],
  ['git colours', { git0: '#ff0000', git7: '#00ff00' }],
  [
    'quadrant and requirement colours',
    { quadrant1Fill: '#123123', requirementBorderColor: 'hsl(300 40% 60%)' },
  ],
  ['hue units', { primaryColor: 'hsl(0.5turn 50% 50%)', lineColor: 'hsl(100grad 50% 50%)' }],
  ['architecture colours', { archEdgeColor: 'red', archGroupBorderWidth: '4px' }],
  ['a string read as an array', { borderColorArray: 'abcdefghijkl', bkgColorArray: 'xyz' }],
  ['an object read as an array', { borderColorArray: { '0': '#010203', length: 2 } }],
  ['an object with a length read as an array', { borderColorArray: { length: 3, '1': '#aabbcc' } }],
  ['a number read as an array', { borderColorArray: 7 }],
  ['a null palette', { borderColorArray: null }],
  ['a null journey palette', { bkgColorArray: null }],
  ['a number as a colour', { primaryColor: 5 }],
  ['a number as a colour read by a function', { background: 0 }],
  ['a null colour', { secondaryColor: null, primaryBorderColor: null }],
  ['a null object override', { xyChart: null }],
  ['a null key of an object override', { xyChart: { backgroundColor: null } }],
  ['calculated secondBkg', { secondBkg: 'calculated' }],
  ['a colour as secondBkg', { secondBkg: '#123456' }],
  ['THEME_COLOR_LIMIT as an array', { THEME_COLOR_LIMIT: [8] }],
  ['THEME_COLOR_LIMIT as true', { THEME_COLOR_LIMIT: true }],
  ['THEME_COLOR_LIMIT as an empty string', { THEME_COLOR_LIMIT: '' }],
  ['THEME_COLOR_LIMIT as an object', { THEME_COLOR_LIMIT: {} }],
  ['THEME_COLOR_LIMIT negative', { THEME_COLOR_LIMIT: -3 }],
  ['darkMode as text', { darkMode: 'yes' }],
];

/** One input of the run: the case it covers, and what the two exporters are given. */
interface ThemeInput {
  id: number;
  source: string;
  theme: string;
  overrides: Record<string, unknown> | null;
}

interface ThemeOutput {
  id: number;
  ok: boolean;
  variables?: unknown;
  error?: string;
}

const root = fileURLToPath(new URL('../../', import.meta.url));
const workDir = `${root}target/port`;
const inputPath = `${workDir}/theme-inputs.json`;
const jsPath = `${workDir}/theme-js.json`;
const rsPath = `${workDir}/theme-rs.json`;

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

/** The first key that differs between two variable objects, for the report. */
function firstDifference(js: unknown, rs: unknown): string {
  const jsObject = (js ?? {}) as Record<string, unknown>;
  const rsObject = (rs ?? {}) as Record<string, unknown>;
  const keys = [...new Set([...Object.keys(jsObject), ...Object.keys(rsObject)])].sort();
  for (const key of keys) {
    if (!Object.hasOwn(jsObject, key)) return `${key}: only in Rust`;
    if (!Object.hasOwn(rsObject, key)) return `${key}: only in the reference`;
    if (!same(jsObject[key], rsObject[key])) {
      return `${key}: reference ${JSON.stringify(jsObject[key])}, Rust ${JSON.stringify(rsObject[key])}`;
    }
  }
  return 'no key differs';
}

const inputs: ThemeInput[] = [];
for (const [label, overrides] of OVERRIDE_CASES) {
  for (const theme of THEMES) {
    inputs.push({ id: inputs.length, source: `${theme}: ${label}`, theme, overrides });
  }
}

await mkdir(workDir, { recursive: true });
await writeFile(inputPath, JSON.stringify(inputs, null, 2));

run('pnpm', ['exec', 'tsx', 'tools/port/export-theme.ts', inputPath, jsPath]);
run('cargo', [
  'run',
  '--quiet',
  '-p',
  'ferrigura-model',
  '--example',
  'theme',
  '--',
  inputPath,
  rsPath,
]);

const js = JSON.parse(await readFile(jsPath, 'utf8')) as ThemeOutput[];
const rs = JSON.parse(await readFile(rsPath, 'utf8')) as ThemeOutput[];
const jsById = new Map(js.map((output) => [output.id, output]));
const rsById = new Map(rs.map((output) => [output.id, output]));

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
    (!jsOutput.ok || same(jsOutput.variables, rsOutput.variables));
  if (agree) continue;
  differences += 1;
  console.log(`\n✗ ${input.source}`);
  if (jsOutput?.ok && rsOutput?.ok) {
    console.log(`  ${firstDifference(jsOutput.variables, rsOutput.variables)}`);
  } else {
    console.log(`  js: ${JSON.stringify(jsOutput).slice(0, 600)}`);
    console.log(`  rs: ${JSON.stringify(rsOutput).slice(0, 600)}`);
  }
}

console.log(
  `\ntheme: ${inputs.length} cases, ${accepted} accepted by the reference, ${differences} differences`
);

await writeFile(
  `${workDir}/theme-summary.json`,
  JSON.stringify({ type: 'theme', inputs: inputs.length, accepted, differences }, null, 2)
);

if (differences > 0) {
  process.exit(1);
}
