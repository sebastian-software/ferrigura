// @vitest-environment node
// util.ts resolves paths from `import.meta.url`, which is not a file:// URL
// under the default jsdom environment.
import { describe, expect, it } from 'vitest';
import { packageOptions } from '../.build/common.js';
import { defaultOptions, getBuildConfig } from './util.js';

const buildFor = (packageName: keyof typeof packageOptions, core: boolean) =>
  getBuildConfig({
    ...defaultOptions,
    core,
    format: 'esm',
    options: packageOptions[packageName],
  });

describe('getBuildConfig externals', () => {
  it('externalizes dependencies in the core build', () => {
    const external = buildFor('mermaid', true).external ?? [];
    expect(external).toContain('elkjs');
    expect(external).toContain('d3');
  });

  it('bundles everything in the non-core build', () => {
    // The esm entry is the self-contained one (standalone + dev server). Only
    // the node built-ins stay external there.
    const external = buildFor('mermaid', false).external ?? [];
    expect(external).not.toContain('elkjs');
    expect(external).not.toContain('d3');
  });
});
