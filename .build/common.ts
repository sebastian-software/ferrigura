export interface PackageOptions {
  name: string;
  packageName: string;
  file: string;
}

/**
 * Shared common options for both ESBuild and Vite
 */
export const packageOptions = {
  parser: {
    name: 'mermaid-parser',
    packageName: 'parser',
    file: 'index.ts',
  },
  mermaid: {
    name: 'mermaid',
    packageName: 'mermaid',
    file: 'mermaid.ts',
  },
} as const satisfies Record<string, PackageOptions>;

/**
 * Minimum runtimes the published bundles support.
 *
 * Keep in sync with the `browserslist` and `engines` fields in the root
 * package.json and with `packages/mermaid/src/docs/config/usage.md`.
 */
export const buildTarget = [
  'es2024',
  'safari17.4',
  'ios17.4',
  'chrome121',
  'edge121',
  'firefox123',
  'node22.12',
] as const;
