import jison from './.vite/jisonPlugin.js';
import jsonSchemaPlugin from './.vite/jsonSchemaPlugin.js';
import typescript from '@rollup/plugin-typescript';
import { defaultExclude, defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: {
    extensions: ['.js'],
  },
  plugins: [
    jison(),
    jsonSchemaPlugin(), // handles .schema.yaml JSON Schema files
    typescript({ compilerOptions: { declaration: false } }),
  ],
  test: {
    environment: 'jsdom',
    globals: true,
    exclude: [
      ...defaultExclude,
      // Worktrees the Claude desktop app creates inside the repository.
      '.claude/**',
      // The Rust port: crates/ferrigura-node/test runs under `node --test`, not vitest.
      'crates/**',
      'target/**',
      './tests/**',
      '**/__mocks__/**',
      '**/generated/',
    ],
    includeSource: ['packages/*/src/**/*.{js,ts}'],
    clearMocks: true,
  },
  build: {
    /** If you set esmExternals to true, this plugins assumes that
     all external dependencies are ES modules */

    commonjsOptions: {
      esmExternals: true,
    },
  },
  define: {
    // Needs to be string
    'injected.profiling': 'false',
    'import.meta.vitest': 'undefined',
    packageVersion: "'0.0.0'",
  },
});
