/**
 * Formats the repository with oxfmt (`.oxfmtrc.jsonc`), leaving out what `.gitignore` leaves out
 * and the generated files below. oxfmt resolves the ignore patterns of its configuration within the
 * configuration's folder, so the files outside `tools/port` are left out here, as negated patterns.
 *
 * Usage: `tsx format.ts [--check] [file …]`, with no file for the whole repository
 * (`pnpm -C tools/port run format`, `format:check`, and the pre-commit hook for the staged files).
 */
import { spawnSync } from 'node:child_process';
import { isAbsolute, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const oxfmt = fileURLToPath(new URL('node_modules/.bin/oxfmt', import.meta.url));

/** The files oxfmt leaves alone, relative to the repository root. */
const generated = [
  // The native npm runtime and release helper are checked byte for byte against Ferramenta.
  'node/ferrigura/native-runtime.mjs',
  'node/ferrigura/native-runtime.d.mts',
  'node/scripts/native-registry.mjs',
  // Generated N-API types and verbatim third-party notices travel with the native package.
  'node/ferrigura/native.d.ts',
  'node/ferrigura/NOTICE.md',
  'node/platforms/*/NOTICE.md',
  // Cargo's manifests, which are Rust's to format, not the harness's
  '**/*.toml',
  // Written by release-please
  'CHANGELOG.md',
  // Written by mdtheme from README.md.src
  'README.md',
  // Written by pnpm
  '**/pnpm-lock.yaml',
  // Generated from elkjs output
  'crates/ferrigura-elk/tests/corpus/**',
  // Generated from Chromium captures of Mermaid 12.2.0's cose-bilkent and fcose layouts
  'crates/ferrigura-cose/tests/corpus/**',
  'crates/ferrigura-cose/tests/fcose-corpus/**',
  // Generated from Chromium's getBBox() by tools/port/path-bounds.ts
  'crates/ferrigura-path/tests/corpus/**',
  // Generated from Chromium captures by tools/port/crates/venn/capture-corpus.ts
  'crates/ferrigura-venn/tests/corpus/**',
  // Generated from the layout stages of Mermaid 12.2.0's swimlane layout
  'crates/ferrigura-swimlane/tests/corpus/**',
  // Generated from reference captures by the M5 port tools (tools/port/)
  'crates/ferrigura-render/tests/data/**',
  'crates/ferrigura-render/tests/fixtures/**',
  // Generated from the Chromium probes by tools/port/text/ (the extents rows are one line each)
  'crates/ferrigura-text/tests/fixtures/**',
  // Vendored from non-layered-tidy-tree-layout 2.0.2 (MIT): kept as published, not reformatted
  'tools/port/vendor/**',
];

const args = process.argv.slice(2);
const check = args.includes('--check');
const files = args
  .filter((arg) => arg !== '--check')
  .map((file) =>
    isAbsolute(file) ? relative(root, file) : relative(root, join(process.cwd(), file))
  );

const result = spawnSync(
  oxfmt,
  [
    check ? '--check' : '--write',
    '-c',
    'tools/port/.oxfmtrc.jsonc',
    '--ignore-path',
    '.gitignore',
    '--no-error-on-unmatched-pattern',
    ...(files.length > 0 ? files : ['.']),
    ...generated.map((pattern) => `!${pattern}`),
  ],
  { cwd: root, stdio: 'inherit' }
);
process.exit(result.status ?? 1);
