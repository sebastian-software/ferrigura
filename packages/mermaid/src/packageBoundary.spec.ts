import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { describe, expect, it } from 'vitest';

const srcDir = __dirname;
const packageDir = resolve(srcDir, '..');

describe('package boundary', () => {
  it('imports other workspace packages by name, not by a path into them', () => {
    // A relative path into a sibling package, say `../../../../parser/dist/...`, works in the
    // monorepo but survives into the published declarations, where nothing is at that path.
    const offenders = readdirSync(srcDir, { recursive: true, encoding: 'utf8' })
      .filter((file) => /\.m?[jt]s$/.test(file))
      .flatMap((file) => {
        const path = join(srcDir, file);
        return [...readFileSync(path, 'utf8').matchAll(/\bfrom\s+'(\.[^']*)'/g)]
          .map(([, specifier]) => specifier)
          .filter((specifier) => !resolve(dirname(path), specifier).startsWith(packageDir + sep))
          .map((specifier) => `${file}: ${specifier}`);
      });

    expect(offenders).toEqual([]);
  });
});
