import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyPackedConsumer } from 'ferramenta-native';
import config from '../native-npm.config.mjs';

// Compile the same public API examples against the installed tarball, not the checkout's types.
const fixture = new URL('../ferrigura/test/types.mts', import.meta.url);
const compiler = fileURLToPath(import.meta.resolve('typescript/bin/tsc'));
await verifyPackedConsumer({
  ...config,
  consumer: {
    ...config.consumer,
    async verify({ consumer, run }) {
      const source = await readFile(fixture, 'utf8');
      const localImport = "from '../index.mjs'";
      assert(source.includes(localImport), 'The TypeScript fixture must import the local entry');
      const installedFixture = join(consumer, 'types.mts');
      const catalogImport = "from '../font-catalog.mjs'";
      assert(
        source.includes(catalogImport),
        'The TypeScript fixture must import the catalog entry'
      );
      await writeFile(
        installedFixture,
        source
          .replace(localImport, "from '@ferrigura/core'")
          .replace(catalogImport, "from '@ferrigura/core/font-catalog'")
      );
      run(process.execPath, [
        '--input-type=module',
        '-e',
        "const m = await import('@ferrigura/core/font-catalog'); if (typeof m.resolveCatalogFonts !== 'function') throw Error('missing catalog export')",
      ]);
      run(process.execPath, [
        compiler,
        '--noEmit',
        '--strict',
        '--module',
        'NodeNext',
        '--moduleResolution',
        'NodeNext',
        '--target',
        'ES2022',
        installedFixture,
      ]);
      console.log('Packed consumer TypeScript examples passed');
    },
  },
});
