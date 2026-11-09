// Measures what each diagram type adds to the Node addon, and writes homepage/app/data/size.json,
// which the homepage's size page draws (homepage/scripts/render-size.mjs).
//
//   node tools/size/measure.mjs [--jobs 4] [--out <file>]
//
// The addon crate has one cargo feature per diagram type (docs/decisions/0020-diagram-types-as-plugins.md).
// The script builds it under the `release-node` profile, the one the npm packages ship, once with each
// type alone, once with a few common sets, and once with every type, and builds the command-line tool
// too. The smallest build, `info` alone, is the core: configuration, theme, text engine with its
// bundled fonts, SVG output and the Node binding. What a type adds is its build's size minus the core.
// Types that share a layout engine each carry it alone, so their additions overlap.
//
// Each build is a full fat-LTO link of a minute or two; `--jobs` builds run at once, each in a
// target directory of its own below target/size, which later runs reuse. The sizes depend on the
// platform and the toolchain, which the output records; the size page names them.
import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { cpus } from 'node:os';
import { gzipSync } from 'node:zlib';

const root = new URL('../../', import.meta.url);
const argument = (name, fallback) => {
  const at = process.argv.indexOf(name);
  return at === -1 ? fallback : process.argv[at + 1];
};
const jobs = Number(argument('--jobs', Math.max(1, Math.floor(cpus().length / 5))));
const out = new URL(argument('--out', 'homepage/app/data/size.json'), root);

// The types, in the order of the addon's `all` feature.
const manifest = readFileSync(new URL('crates/ferrigura-node/Cargo.toml', root), 'utf8');
const all = /^all = \[([^\]]*)\]/m.exec(manifest)?.[1];
if (!all) throw new Error('crates/ferrigura-node/Cargo.toml has no `all` feature');
const types = [...all.matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]);

const library =
  { darwin: 'libferrigura_node.dylib', win32: 'ferrigura_node.dll' }[process.platform] ??
  'libferrigura_node.so';
const cliBinary = process.platform === 'win32' ? 'ferrigura.exe' : 'ferrigura';

/** The size of a file and of its gzip at the highest level, in bytes. */
function sizes(file) {
  const bytes = readFileSync(file);
  return { bytes: statSync(file).size, gzip: gzipSync(bytes, { level: 9 }).length };
}

/** Runs cargo and resolves when it exits; its output goes to the log of its slot. */
function cargo(args, slot) {
  const targetDir = new URL(`target/size/${slot}/`, root);
  mkdirSync(targetDir, { recursive: true });
  return new Promise((resolve, reject) => {
    const child = spawn('cargo', args, {
      cwd: root,
      env: { ...process.env, CARGO_TARGET_DIR: targetDir.pathname },
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    let log = '';
    child.stderr.on('data', (chunk) => (log += chunk));
    child.on('exit', (code) =>
      code === 0 ? resolve(targetDir) : reject(new Error(`cargo ${args.join(' ')}\n${log}`))
    );
  });
}

const sets = [
  { label: 'Sequence diagrams', features: ['sequence'] },
  { label: 'Flowcharts', features: ['flowchart'] },
  { label: 'Five common types', features: ['flowchart', 'sequence', 'class', 'state', 'er'] },
  { label: 'Every type', features: ['all'] },
];
const builds = [
  ...types.map((type) => ({ key: type, features: [type] })),
  ...sets
    .filter(({ features }) => features.length > 1 || features[0] === 'all')
    .map(({ features }) => ({
      key: features.join(','),
      features,
    })),
];

const results = new Map();
let next = 0;
async function worker(slot) {
  while (next < builds.length) {
    const build = builds[next++];
    const dir = await cargo(
      [
        'build',
        '--profile',
        'release-node',
        '-p',
        'ferrigura-node',
        '--no-default-features',
        '--features',
        build.features.join(','),
      ],
      slot
    );
    results.set(build.key, sizes(new URL(`release-node/${library}`, dir)));
    process.stderr.write(`${results.size}/${builds.length} ${build.key}\n`);
  }
}
await Promise.all(Array.from({ length: jobs }, (_, slot) => worker(`slot-${slot}`)));
const cliDir = await cargo(['build', '--release', '-p', 'ferrigura-cli'], 'slot-0');

const core = results.get('info');
const text = (command, args) => execFileSync(command, args, { cwd: root, encoding: 'utf8' }).trim();
const report = {
  measuredWith: {
    commit: text('git', ['rev-parse', '--short', 'HEAD']),
    rustc: text('rustc', ['--version']),
    platform: `${process.platform}-${process.arch}`,
    profile: 'release-node',
  },
  cli: sizes(new URL(`release/${cliBinary}`, cliDir)),
  core,
  types: types
    .filter((type) => type !== 'info')
    .map((type) => {
      const { bytes, gzip } = results.get(type);
      return { type, bytes, gzip, adds: bytes - core.bytes, addsGzip: gzip - core.gzip };
    }),
  sets: sets.map(({ label, features }) => ({
    label,
    features,
    ...results.get(
      features.length === 1 && features[0] !== 'all' ? features[0] : features.join(',')
    ),
  })),
};
writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`);
process.stderr.write(`wrote ${out.pathname}\n`);
