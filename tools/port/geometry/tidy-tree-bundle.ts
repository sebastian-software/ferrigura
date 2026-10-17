/**
 * The reference of `layout: tidy-tree`: the package `@mermaid-js/layout-tidy-tree` 1.0.1, which the
 * fork removed in `2d85261d6`. Its four sources are read from the parent commit (`git show`), and
 * bundled with esbuild into one ESM file under `target/geometry/tidy-tree/`, with the library
 * `non-layered-tidy-tree-layout` 2.0.2 from `tools/port/vendor/` (MIT, Copyright (c) 2019 Michael
 * Wong). Nothing of the package is vendored; the library is, so that no dependency enters the lock
 * file.
 *
 * The bundle exports the package's layouts, which the page registers with
 * `mermaid.registerLayoutLoaders`, and sets `globalThis.__tidyTree` to `{ executeTidyTreeLayout }`,
 * for captures of generated trees. Its copy of `render.ts` has one line added after the layout call:
 * `globalThis.__mermaidTidyTreeCapture(input, result)`, with the input the renderer hands over
 * (`updatedLayoutData`, render.ts 93-105). The insertion is checked to match exactly once, so the
 * capture cannot silently go missing when the source changes.
 *
 * The bundle is built once per process, when a fixture that asks for the layout is rendered.
 */
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { build, type Plugin } from 'esbuild';

const root = fileURLToPath(new URL('../../../', import.meta.url));

/** The directory of the bundle and of its sources, under the harness's work directory. */
export const TIDY_TREE_DIR = join(root, 'target/geometry/tidy-tree');
/** The bundle's file, and the URL the page imports it from. */
export const TIDY_TREE_BUNDLE = join(TIDY_TREE_DIR, 'tidy-tree.mjs');
export const TIDY_TREE_URL = '/tidy-tree/tidy-tree.mjs';

/** The removed package's last commit: its parent is the one that deleted the sources. */
const REMOVED_IN = '2d85261d6';
const PACKAGE_SOURCES = 'packages/mermaid-layout-tidy-tree/src';
const SOURCE_FILES = ['layout.ts', 'render.ts', 'layouts.ts', 'types.ts'];

/** The vendored library: its `index.js` exports `layout`, `Tree`, `BoundingBox` and `Layout`. */
const LIBRARY = join(root, 'tools/port/vendor/non-layered-tidy-tree-layout/index.js');
const LIBRARY_NAME = 'non-layered-tidy-tree-layout';

/** The line of `render.ts` after which the capture hook goes (render.ts line 105). */
const LAYOUT_CALL = '  const layoutResult = await executeTidyTreeLayout(updatedLayoutData);\n';

/** The hook: the renderer's input and the layout's result, as the harness reads them. */
const CAPTURE_HOOK = `  (globalThis as unknown as { __mermaidTidyTreeCapture?: (input: unknown, result: unknown) => void }).__mermaidTidyTreeCapture?.(
    {
      nodes: updatedLayoutData.nodes.map((node) => ({ id: node.id, width: node.width, height: node.height, shape: node.shape })),
      edges: updatedLayoutData.edges.map((edge) => ({ id: edge.id, start: edge.start, end: edge.end })),
    },
    layoutResult
  );
`;

/** The entry: the package's layouts, and the layout function for the generated-tree captures. */
const ENTRY = `import tidyTreeLayouts from './src/layouts.ts';
import { executeTidyTreeLayout } from './src/layout.ts';

(globalThis as unknown as { __tidyTree: unknown }).__tidyTree = { executeTidyTreeLayout };

export default tidyTreeLayouts;
`;

/** The package's source at the commit before the removal. */
function sourceAtRemoval(file: string): string {
  return execFileSync('git', ['show', `${REMOVED_IN}^:${PACKAGE_SOURCES}/${file}`], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
  });
}

/** `render.ts` with the capture hook after its layout call, checked to match exactly once. */
function withCaptureHook(render: string): string {
  const occurrences = render.split(LAYOUT_CALL).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `render.ts has ${occurrences} layout calls matching the capture hook's anchor, expected 1`
    );
  }
  return render.replace(LAYOUT_CALL, LAYOUT_CALL + CAPTURE_HOOK);
}

/** Resolves the library's package name to the vendored copy. */
const vendoredLibrary: Plugin = {
  name: 'vendored-tidy-tree-library',
  setup(buildContext) {
    buildContext.onResolve({ filter: new RegExp(`^${LIBRARY_NAME}$`) }, () => ({ path: LIBRARY }));
  },
};

let bundling: Promise<string> | null = null;

/**
 * Builds the bundle, once per process, and returns its file. The sources are extracted from the
 * history on every build, so the bundle is the removed package as it was committed.
 */
export function ensureTidyTreeBundle(): Promise<string> {
  bundling ??= buildBundle();
  return bundling;
}

async function buildBundle(): Promise<string> {
  const sourceDir = join(TIDY_TREE_DIR, 'src');
  await mkdir(sourceDir, { recursive: true });
  for (const file of SOURCE_FILES) {
    const text =
      file === 'render.ts' ? withCaptureHook(sourceAtRemoval(file)) : sourceAtRemoval(file);
    await writeFile(join(sourceDir, file), text);
  }
  const entry = join(TIDY_TREE_DIR, 'entry.ts');
  await writeFile(entry, ENTRY);
  await build({
    entryPoints: [entry],
    bundle: true,
    format: 'esm',
    platform: 'browser',
    target: 'es2022',
    outfile: TIDY_TREE_BUNDLE,
    plugins: [vendoredLibrary],
    logLevel: 'warning',
  });
  return TIDY_TREE_BUNDLE;
}
